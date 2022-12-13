use phf::phf_map;
use xcb::x::KeyButMask;
use nix::pty::openpty;
use std::fs::File;
use std::os::unix::io::FromRawFd;
use std::process::{Child, Command};
use std::env;
use super::state::Config;

pub struct Pty {
    process: Child,
    pub fd: File,
}
impl Pty {
    pub fn new(config: &Config) -> Result<Pty,Box<dyn std::error::Error>> {
	let winsize = nix::pty::Winsize {
	    ws_row: config.n_lines,
	    ws_col: config.line_width,
	    ws_xpixel: config.line_width,
	    ws_ypixel: config.n_lines,
	};
	let ends = openpty(&winsize,None)?;
	let (master, slave) = (ends.master, ends.slave);
	let shell = env::var("SHELL")?;
	let mut shell_builder = Command::new(shell);
	let builder = shell_builder.env("TERM","pcansi");
	
	builder.stdin(unsafe {File::from_raw_fd(slave)});
	builder.stdout(unsafe {File::from_raw_fd(slave)});
	builder.stderr(unsafe {File::from_raw_fd(slave)});

	Ok(Self {
	    process: builder.spawn()?,
	    fd: unsafe {File::from_raw_fd(master)},
	})
	
    }
    pub fn parse_key(key_sym: xkbcommon::xkb::Keysym, mask: KeyButMask, key_sym_mod: xkbcommon::xkb::Keysym)
		     -> String {
	if let Some(shortcuts) = SHORTCUTS.get(&xkbcommon::xkb::keysym_get_name(key_sym)) {
	    for (msk,fix,val) in *shortcuts {
		if mask & *fix == *msk {
		    //println!("shortcut matched: {:?}",val.as_bytes());
		    return String::from(*val);
		}
	    }
	}
	let mut utf8 = xkbcommon::xkb::keysym_to_utf8(key_sym_mod);
	utf8.pop();
	utf8
    }
}


const FIX: KeyButMask = KeyButMask::from_bits_truncate(
    KeyButMask::SHIFT.bits()|
    KeyButMask::LOCK.bits()|
    KeyButMask::CONTROL.bits()|
    KeyButMask::MOD1.bits()|
    KeyButMask::MOD3.bits()|
    KeyButMask::MOD4.bits()|
    KeyButMask::MOD5.bits());
const NONE: KeyButMask = KeyButMask::empty();
const SHIFT: KeyButMask = KeyButMask::SHIFT;
const CONTROL: KeyButMask = KeyButMask::CONTROL;
const MOD1: KeyButMask = KeyButMask::MOD1;
static SHORTCUTS: phf::Map<&'static str,&'static [&'static (KeyButMask,KeyButMask,&'static str)]> = phf_map! {
    "KP_Home" => &[&(SHIFT,FIX,"\x1b[2J"),
		   &(NONE,NONE,"\x1b[H")],
    "KP_Up" => &[&(NONE,NONE,"\x1b[A")],
    "KP_Down" => &[&(NONE,NONE,"\x1b[B")],
    "KP_Left" => &[&(NONE,NONE,"\x1b[D")],
    "KP_Right" => &[&(NONE,NONE,"\x1b[C")],
    "KP_Prior" => &[&(SHIFT,FIX,"\x1b[5;2~"),
		    &(NONE,NONE,"\x1b[5~")],
    "KP_Begin" => &[&(NONE,NONE,"\x1b[E")],
    "KP_End" => &[&(CONTROL, FIX, "\x1b[J"),
		  &(SHIFT, FIX, "\x1b[K"),
		  &(NONE,NONE,"\x1b[4~")],
    "KP_Next" => &[&(SHIFT, FIX, "\x1b[6;2~"),
		   &(NONE,NONE,"\x1b[6~")],
    "KP_Insert" => &[&(SHIFT, FIX, "\x1b[4l"),
		     &(CONTROL, FIX, "\x1b[L"),
		     &(NONE,NONE,"\x1b[4h")],
    "KP_Delete" => &[&(CONTROL, FIX, "\x1b[M"),
		     &(SHIFT, FIX, "\x1b[2K"),
		     &(NONE,NONE,"\x1b[P")],
    "KP_Enter" => &[&(NONE,NONE,"\n")],   // \r in st for some reason
    "Up" => &[&(SHIFT, FIX, "\x1b[1;2A"),
	      &(MOD1, FIX, "\x1b[1;3A"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | MOD1.bits()), FIX, "\x1b[1;4A"),
	      &(CONTROL, FIX, "\x1b[1;5A"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits()), FIX, "\x1b[1;6A"),
	      &(KeyButMask::from_bits_truncate(CONTROL.bits() | MOD1.bits()), FIX, "\x1b[1;7A"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits() | MOD1.bits()),FIX, "\x1b[1;8A"),
	      &(NONE,NONE,"\x1b[A")],
    "Down" => &[&(SHIFT, FIX, "\x1b[1;2B"),
	      &(MOD1, FIX, "\x1b[1;3B"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | MOD1.bits()), FIX, "\x1b[1;4B"),
	      &(CONTROL, FIX, "\x1b[1;5B"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits()), FIX, "\x1b[1;6B"),
	      &(KeyButMask::from_bits_truncate(CONTROL.bits() | MOD1.bits()), FIX, "\x1b[1;7B"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits() | MOD1.bits()),FIX, "\x1b[1;8B"),
	      &(NONE,NONE,"\x1b[B")],
    "Left" => &[&(SHIFT, FIX, "\x1b[1;2D"),
	      &(MOD1, FIX, "\x1b[1;3D"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | MOD1.bits()), FIX, "\x1b[1;4D"),
	      &(CONTROL, FIX, "\x1b[1;5D"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits()), FIX, "\x1b[1;6D"),
	      &(KeyButMask::from_bits_truncate(CONTROL.bits() | MOD1.bits()), FIX, "\x1b[1;7D"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits() | MOD1.bits()),FIX, "\x1b[1;8D"),
	      &(NONE,NONE,"\x1b[D")],
    "Right" => &[&(SHIFT, FIX, "\x1b[1;2C"),
	      &(MOD1, FIX, "\x1b[1;3C"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | MOD1.bits()), FIX, "\x1b[1;4C"),
	      &(CONTROL, FIX, "\x1b[1;5C"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits()), FIX, "\x1b[1;6C"),
	      &(KeyButMask::from_bits_truncate(CONTROL.bits() | MOD1.bits()), FIX, "\x1b[1;7C"),
	      &(KeyButMask::from_bits_truncate(SHIFT.bits() | CONTROL.bits() | MOD1.bits()),FIX, "\x1b[1;8C"),
	      &(NONE,NONE,"\x1b[C")],
    "Tab" => &[&(KeyButMask::SHIFT,FIX,"\x1b[Z")],
    "Return" => &[&(KeyButMask::MOD1,FIX,"\x1b\n"), // \r in st
		  &(NONE,NONE,"\n")],  // \r in st
    "Insert" => &[&(KeyButMask::SHIFT, FIX, "\x1b[4l"),
		  &(KeyButMask::CONTROL, FIX, "\x1b[L"),
		  &(NONE,NONE,"\x1b[4h")],
    "Delete" => &[&(KeyButMask::CONTROL, FIX, "\x1b[M"),
		  &(KeyButMask::SHIFT, FIX, "\x1b[2K"),
		  &(NONE,NONE,"\x1b[P")],
    "BackSpace" => &[&(NONE,FIX,"\x7f"),
		     &(KeyButMask::MOD1,FIX,"\x1b\x7f")],
    "Home" => &[&(KeyButMask::SHIFT,FIX,"\x1b[2J"),
		&(NONE,NONE,"\x1b[H")],
    "End" => &[&(KeyButMask::CONTROL, FIX, "\x1b[J"),
	       &(KeyButMask::SHIFT, FIX, "\x1b[K"),
	       &(NONE,NONE,"\x1b[4~")],
    "Prior" => &[&(KeyButMask::CONTROL,FIX,"\x1b[5;5~"),
		 &(KeyButMask::SHIFT,FIX,"\x1b[5;2~"),
		 &(NONE,NONE,"\x1b[5~")],
    "Next" => &[&(KeyButMask::CONTROL, FIX, "\x1b[6;5~"),
		&(KeyButMask::SHIFT, FIX, "\x1b[6;2~"),
		&(NONE,NONE,"\x1b[6~")],
};
