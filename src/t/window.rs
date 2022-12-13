use xcb::{x,Connection};
use xkbcommon::xkb;
use super::state::{Config,WindowPosition};
use super::State;

impl WindowPosition {
    fn get_xy(&self, width: u16, height: u16, screen: &x::Screen) -> (i32,i32) {
	(
	    if self.right {
		screen.width_in_pixels()-self.x_offset-width
	    } else { self.x_offset } as i32,
	    if self.bottom {
		screen.height_in_pixels()-self.y_offset-height
	    } else { self.y_offset } as i32
	)
    }
}
pub struct Window {
    pub conn: Connection,
    pub window: x::Window,
    pub gc: x::Gcontext,
    screen: Box<x::ScreenBuf>,
    font: x::Charinfo,
    pub xkb_state: xkb::State,
    xkb_state_nomod: xkb::State,
}
impl Window {
    pub fn get_keysym(&self, ev: &x::KeyPressEvent) -> xkb::Keysym {
	self.xkb_state_nomod.key_get_one_sym(ev.detail().into())
    }
    pub fn get_keysym_mod(&mut self, ev: &x::KeyPressEvent) -> xkb::Keysym {
	self.xkb_state.key_get_one_sym(ev.detail().into())
    }
    pub fn reload_keymap(&mut self) {
	self.xkb_state = Self::reload_xkb_map(&self.conn);
	self.xkb_state_nomod = self.xkb_state.clone();
    }
    pub fn redraw<T>(&mut self, state: &State<T>)
    where
	T: FnMut(Option<usize>)
    {
	let l_h = 3+self.font.ascent+self.font.descent;
	//println!("redrawing lines: {:?}",state.redraw_lines);
	if let Some(numbers) = &state.redraw_lines {
	    let (white,black) = (self.screen.white_pixel(),self.screen.black_pixel());
	    let (fg,bg) = if state.config.dark {(white,black)}else{(black,white)};
	    
	    self.conn.send_request(&x::ChangeGc { gc: self.gc, value_list: &[
		x::Gc::Foreground(bg)],});
	    for n in numbers {
		self.conn.send_request(&x::PolyFillRectangle {
		    drawable: x::Drawable::Window(self.window),
		    gc: self.gc,
		    rectangles: &[x::Rectangle {
			x: 3, y: 3 + l_h*(*n+state.lines.start) as i16,                                        // 
			width: state.config.line_width*self.font.character_width as u16,
			height: (self.font.ascent+self.font.descent) as u16,
		    }],
		});
	    }
	    self.conn.send_request(&x::ChangeGc { gc: self.gc, value_list: &[
		x::Gc::Foreground(fg)],});
	    for n in numbers {
		let cur = if *n == state.lines.cur() {Some(state.col as u16)} else {None};
		self.draw_text_line(&state.lines[*n+state.lines.start], *n, cur, state.config.line_width);  // 
	    }
	} else {
	    self.apply(&state.config, state.lines.vis_len());
	    for n in 0..state.lines.vis_len() {
		let cur = if n == state.lines.cur() {Some(state.col as u16)} else {None};
		self.draw_text_line(&state.lines[n+state.lines.start], n, cur, state.config.line_width);  // 
	    }
	}
	
    }
    pub fn new<T>(state: &State<T>) -> Result<Window, Box<dyn std::error::Error>>
    where
	T: FnMut(Option<usize>)
    {
	let (conn, screen_num) = Connection::connect(None)?;

	let font: x::Font = conn.generate_id();
	conn.send_request(&x::OpenFont {
	    fid: font,
	    name: b"fixed"});
	let font_cookie = conn.send_request(&x::QueryFont {
	    font: xcb::x::Fontable::Font(font)});
	let font_info = conn.wait_for_reply(font_cookie)?;
	let font_max_info = font_info.max_bounds();
	if font_info.min_bounds().character_width != font_max_info.character_width {
	    conn.send_request(&x::CloseFont{font});
	    Err("dynamically spaced fonts are not supported!")?;
	}
	
	let setup = conn.get_setup();
	let screen = setup.roots().nth(screen_num as usize).unwrap().to_owned();

	let window: x::Window = conn.generate_id();
	let w_cookie = conn.send_request_checked(&x::CreateWindow {
	    depth: x::COPY_FROM_PARENT as u8,
	    wid: window,
	    parent: screen.root(),
	    x: 0,
	    y: 0,
	    width: 10,
	    height: 10,
	    border_width: 1,
	    class: x::WindowClass::InputOutput,
	    visual: screen.root_visual(),
	    value_list: &[
		x::Cw::OverrideRedirect(true),
		x::Cw::EventMask(x::EventMask::KEY_PRESS |
				 x::EventMask::KEY_RELEASE |
				 x::EventMask::FOCUS_CHANGE),],
	});
	conn.check_request(w_cookie)?;
	
	let gc: x::Gcontext = conn.generate_id();
	let gc_cookie = conn.send_request_checked(&x::CreateGc {
	    cid: gc,
	    drawable: x::Drawable::Window(window),
	    value_list: &[
		x::Gc::Font(font),
	    ],
	});
	conn.check_request(gc_cookie)?;
	conn.send_request_checked(&x::CloseFont {font} );

	Self::setup_xkb(&conn);
	let xkb_state = Self::reload_xkb_map(&conn);
	let xkb_state_nomod = xkb_state.clone();
	
	let mut my_window_instance = Window {
	    conn: conn,
	    window: window,
	    gc: gc,
	    screen: Box::new(screen),
	    font: font_max_info,
	    xkb_state: xkb_state,
	    xkb_state_nomod: xkb_state_nomod
	};

	my_window_instance.conn.send_request(&x::MapWindow {window});
	my_window_instance.redraw(state);
	my_window_instance.conn.send_request(&x::SetInputFocus {
	    revert_to: x::InputFocus::PointerRoot,
	    focus: window,
	    time: x::CURRENT_TIME,
	});
	my_window_instance.conn.flush()?;
	Ok(my_window_instance)
    }
    fn apply(&mut self, config: &Config, number_lines: usize) {
	let (white,black) = (self.screen.white_pixel(),self.screen.black_pixel());
	let (fg,bg) = if config.dark {(white,black)}else{(black,white)};
	let width: u16 = 6+config.line_width*self.font.character_width as u16;
	let height: u16 = 3+number_lines as u16*(3+self.font.ascent+self.font.descent) as u16;
	let (x,y) = config.pos.get_xy(width, height, &self.screen);
	
	self.conn.send_request(&x::ConfigureWindow {
	    window: self.window,
	    value_list: &[
		x::ConfigWindow::X(x),
		x::ConfigWindow::Y(y),
		x::ConfigWindow::Width(width.into()),
		x::ConfigWindow::Height(height.into()),
	    ],
	});
	self.conn.send_request(&x::ChangeWindowAttributes {
	    window: self.window,
	    value_list: &[x::Cw::BackPixel(bg), x::Cw::BorderPixel(fg),],
	});
	self.conn.send_request(&x::ChangeGc {
	    gc: self.gc,
	    value_list: &[x::Gc::Foreground(fg),
			  x::Gc::Background(bg)],
	});

	self.conn.send_request(&x::ClearArea {
	    exposures: false,
	    window: self.window,
	    x: 0,y: 0,width: width, height: height,
	});
	if config.last_line_border && number_lines > 1 {
	    self.conn.send_request(&x::PolyLine {
		coordinate_mode: x::CoordMode::Previous,
		drawable: x::Drawable::Window(self.window),
		gc: self.gc,
		points: &[x::Point {x: 3, y: height as i16 -5-self.font.ascent-self.font.descent},
			  x::Point {x: width as i16-6,y: 0}],
	    });
	}
	
    }
    fn draw_text_line(&mut self, line: &[u8], row: usize, cur: Option<u16>, line_width: u16) {
	let lpad = 5;
	let line_len = line.len() as u16;
	let (start, end, offs) = {
	    if line_len<=line_width {(0,line_len,0)}
	    else if let Some(cur) = &cur {
		if line_len < lpad+*cur {(line_len-line_width+1,line_len,self.font.character_width)}
		else if *cur+lpad <= line_width+1 {(0,line_width-1,0)}
		else {(*cur+lpad-line_width, *cur+lpad-2,self.font.character_width)}
	    } else {(0,line_width-1,0)}
	};
	
	self.conn.send_request(&x::ImageText8 {
	    drawable: x::Drawable::Window(self.window),
	    gc: self.gc,
	    x: 3 + offs,
	    y: 3 + self.font.ascent + (3+self.font.ascent+self.font.descent)*row as i16,
	    string: &line[start as usize..end as usize],
	});
	if let Some(cur) = cur {
	    self.conn.send_request(&x::PolyFillRectangle {
		drawable: x::Drawable::Window(self.window),
		gc: self.gc,
		rectangles: &[x::Rectangle {
		    x: 3+self.font.character_width*(cur as i16-start as i16)+offs,
		    y: 3 + (3+self.font.ascent+self.font.descent)*row as i16,
		    width: self.font.character_width as u16,
		    height: (self.font.ascent+self.font.descent) as u16,
		}],
	    });
	}
    }
    fn setup_xkb(conn: &xcb::Connection) {
	xcb::xkb::prefetch_extension_data(conn);
	if let None = xcb::xkb::get_extension_data(conn) {
	    panic!("XKB extension not supported by X server!");
	}
	let cookie = conn.send_request(&xcb::xkb::UseExtension {wanted_major: 1,wanted_minor: 0});
	match conn.wait_for_reply(cookie) {
	    Ok(r) => {
		if !r.supported() {
		    panic!("xkb-1.0 not supported");
		}
	    },
	    Err(_) => {
		panic!("could not get xkb extension supported version");
	    },
	};
	let map_parts =
	    xcb::xkb::MapPart::KEY_TYPES |
	xcb::xkb::MapPart::KEY_SYMS |
	xcb::xkb::MapPart::MODIFIER_MAP |
	xcb::xkb::MapPart::EXPLICIT_COMPONENTS |
	xcb::xkb::MapPart::KEY_ACTIONS |
	xcb::xkb::MapPart::KEY_BEHAVIORS |
	xcb::xkb::MapPart::VIRTUAL_MODS |
	xcb::xkb::MapPart::VIRTUAL_MOD_MAP;
	
	let events = xcb::xkb::EventType::MAP_NOTIFY;
	
	let cookie = conn.send_request_checked(&xcb::xkb::SelectEvents {
	    device_spec: xcb::xkb::Id::UseCoreKbd as xcb::xkb::DeviceSpec,
	    affect_which: events,
	    clear: xcb::xkb::EventType::empty(),
	    select_all: events,
	    affect_map: map_parts,
	    map: map_parts,
	    details: &[xcb::xkb::SelectEventsDetails::IndicatorMapNotify {
		affect_indicator_map: 0,
		indicator_map_details: 0,
	    }],
	});
	conn.check_request(cookie).expect("failed to select notify events from xcb xkb");
    }
    fn reload_xkb_map(conn: &xcb::Connection) -> xkb::State {
	let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
	let id = xkb::x11::get_core_keyboard_device_id(conn);
	let keymap = xkb::x11::keymap_new_from_device(
	    &context, conn, id, xkb::KEYMAP_COMPILE_NO_FLAGS);
	xkb::x11::state_new_from_device(&keymap, conn, id)
    }
}
