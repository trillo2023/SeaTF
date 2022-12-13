use sea_tf::t;
use std::{io::Write,io::Read,time::Duration,thread};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state: t::State<'_,Box<dyn FnMut(Option<usize>)>> = t::State::new();
    //let _ = state.write_all(b"Hello World!\n");
    let mut window = t::Window::new(&state)?;
    let mut pty = t::Pty::new(&state.config)?;

    
    let dt = Duration::from_millis(100);
    let mut timer = 10;
    let mut read_buf = [0;512];

    loop {
	let (new_timer, ev) = Event::wait_for_event(timer, dt, &mut window.conn, &pty.fd)?;
	timer = new_timer;
	match ev {
	    Event::X(xcb::Event::X(xcb::x::Event::FocusOut(_ev))) => {
		window.conn.send_request(&xcb::x::FreeGc {gc: window.gc});
		break;
	    },
	    Event::X(xcb::Event::X(xcb::x::Event::KeyPress(ev))) => {
		window.xkb_state.update_key(ev.detail().into(),xkbcommon::xkb::KeyDirection::Down);
		let key_sym = window.get_keysym(&ev);
		let key_sym_mod = window.get_keysym_mod(&ev);
		//println!("key down: {}",xkbcommon::xkb::keysym_get_name(key_sym));
		let tmp = t::Pty::parse_key(key_sym,ev.state(),key_sym_mod);
		if tmp.len() != 0 {
		    let _ = pty.fd.write_all(&tmp.as_bytes());
		    timer = 10;
		    //println!("sent: {}",tmp);
		}
	    },
	    Event::X(xcb::Event::X(xcb::x::Event::KeyRelease(ev))) => {
		window.xkb_state.update_key(ev.detail().into(),xkbcommon::xkb::KeyDirection::Up);
	    },
	    Event::T() => {
		let n = pty.fd.read(&mut read_buf)?;
		state.do_and_redraw(|st| {let _ = st.write_all(&read_buf[0..n]);}, &mut window);
		window.conn.flush()?;
	    },
	    _ => (),
	}
	
    }
    Ok(())
}
enum Event {
    X(xcb::Event),
    T(),
}
impl Event {
    fn read_ready(file: &std::fs::File) -> bool {
	use std::os::unix::io::AsRawFd;
	let mut set = nix::sys::select::FdSet::new();
	set.insert(file.as_raw_fd());
	let mut timeout = nix::sys::time::TimeVal::new(0,100);
	nix::sys::select::select(None,&mut set,None,None,&mut timeout) != Ok(0)
    }
    // (timer, event) = wait_for_event(timer, dt, conn, file)?;
    pub fn wait_for_event(timer: u32, dt: Duration, conn: &xcb::Connection, pty: &std::fs::File)
			  -> Result<(u32, Event), Box<dyn std::error::Error>> {
	
	for time in 0..timer {
	    if Self::read_ready(pty) {
		let t = if time == 0 {timer} else {timer+1-time};
		return Ok((t, Self::T()));
	    }
	    if let Some(event) = conn.poll_for_event()? {
		return Ok((timer-time, Self::X(event)));
	    }
	    thread::sleep(dt);
	}
	Ok((0, Event::X(conn.wait_for_event()?)))
    }
}
