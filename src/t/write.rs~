use std::cmp;
use super::state::QueueView;
use super::State;

#[derive(Clone)]
pub(super) struct EscSeq {
    pub esc: Esc,
    pub seq: Vec<u8>,
    pub inv: bool,
}
#[derive(PartialEq,Copy,Clone,Debug)]
pub(super) enum Esc {
    None,
    Esc,
    CSI,
    NF,
    STT,
}
pub(super) trait PreWrite {
    fn pre_write(&mut self, arg: &[u8], pre: &mut EscSeq) -> usize;
}

trait WriteANSI {
    fn write_ansi(&mut self, esc_seq: &EscSeq);
}
impl<T: WriteANSI + WriteASCII> PreWrite for T {
    fn pre_write(&mut self, arg: &[u8], pre: &mut EscSeq) -> usize{
	let mut i: usize = 0;
	if arg.is_empty() {return 0};
	if pre.esc == Esc::Esc {
	    pre.esc = match arg[i] {
		b'[' => { i+=1;  Esc::CSI },
		b'\\' | b'7' | b'8' | b'c' | b'N' | b'O' => { i+=1;  Esc::None },
		other => if other & 0xf0 == 0x20 { Esc::NF } else { Esc::STT },
	    }
	} else {
	    let l = arg[i..].into_iter().take_while(match pre.esc {
		Esc::None => Box::new(|b: &&u8| **b >= 0x20 && **b != 0x7f)
		    as Box<dyn Fn(&&u8)->bool>,
		Esc::CSI => Box::new(|b: &&u8| **b & 0xe0 == 0x20),
		Esc::NF => Box::new(|b: &&u8| **b & 0xf0 == 0x20),
		Esc::STT => Box::new(|b: &&u8| **b != 0x1b),
		Esc::Esc => panic!("unexpected Esc type change!"),
	    }).copied().map(if pre.esc==Esc::None && pre.inv {Box::new(|b: u8| match b & 0xc0 {
		0x80 => (),                   // utf8 tail
		0xc0 => pre.seq.push(b'?'),   // utf8 head
		_ if b == b' ' => pre.seq.push(b'#'),
		_ => pre.seq.push(b),        // ascii char
	    }) as Box<dyn FnMut(u8)>} else {Box::new(|b: u8| match b & 0xc0 {
		0x80 => (),                   // utf8 tail
		0xc0 => pre.seq.push(b'?'),   // utf8 head
		_ => pre.seq.push(b),        // ascii char
	    })}).count();
	    if l == 0 {
		i += match pre.esc {
		    Esc::None => {
			self.write_ascii(match arg[i] {
			    0x1b => {
				pre.esc=Esc::Esc;
				EscASCII::None
			    },
			    0x07 => EscASCII::BEL,
			    0x08 => EscASCII::BS,
			    0x09 => EscASCII::HT,
			    0x0a => EscASCII::LF,
			    0x0d => EscASCII::CR,
			    0x7f => EscASCII::DEL,
			    _ => EscASCII::None,
			});
			1
		    },
		    Esc::CSI | Esc::NF => {
			pre.seq.push(arg[i]);
			self.write_ansi(pre);
			pre.esc = Esc::None;
			pre.seq = Vec::new();
			1
		    },
		    Esc::STT => {
			self.write_ansi(pre);
			pre.esc = Esc::None;
			pre.seq = Vec::new();
			0
		    },
		    Esc::Esc => panic!("unexpected Esc type change!"),
		};
	    } else if pre.esc == Esc::None {
		self.write_ansi(pre);
		pre.seq = Vec::new();
	    }
	    i+=l;
	}
	i
    }
}

#[cfg(test)]
mod pre_write_tests {
    use super::*;
    use std::io;
    use std::io::Write;
    #[test]
    fn csi_interrupted() {
	let mut st = PreWriteTest::new();
	st.write_all("\x1b".as_bytes());
	st.write_all("[A\x1b[".as_bytes());
	st.write_all("13B\x1b[F".as_bytes());
	st.write_all("hallo".as_bytes());
	assert_eq!(st.res, "<No ASCII><CSI:>A<No ASCII><CSI:>13B<No ASCII><CSI:>F<TXT:>hallo");
    }
    #[test]
    fn basic_csi() {
	let mut st = PreWriteTest::new();
	st.write_all("1\x1b[13;5fb".as_bytes());
	assert_eq!(st.res, "<TXT:>1<No ASCII><CSI:>13;5f<TXT:>b");
    }
    #[test]
    fn basic_text() {
	let mut st = PreWriteTest::new();
	st.write_all("hallo".as_bytes());
	assert_eq!(st.res, "<TXT:>hallo");
    }
    #[test]
    fn ascii_control_characters() {
	let mut st = PreWriteTest::new();
	st.write_all("\x01\x07\x08\x09\x0a\x0c\x0d\x7f".as_bytes());
	assert_eq!(st.res, "<No ASCII><BEL><BS><HT><LF><No ASCII><CR><DEL>");
    }
    struct PreWriteTest {
	res: String,
	leftover: EscSeq,
    }
    impl PreWriteTest {
	fn new() -> PreWriteTest {
	    PreWriteTest {
		res: String::new(),
		leftover: EscSeq {
		    esc: Esc::None,
		    seq: Vec::new(),
		    inv: false,
		},
	    }
	}
    }
    impl WriteASCII for PreWriteTest {
	fn write_ascii(&mut self, esc: EscASCII) {
	    self.res += match esc {
		EscASCII::None => "<No ASCII>",
		EscASCII::BEL => "<BEL>",
		EscASCII::BS => "<BS>",
		EscASCII::HT => "<HT>",
		EscASCII::LF => "<LF>",
		EscASCII::CR => "<CR>",
		EscASCII::DEL => "<DEL>",
	    };
	}
    }
    impl WriteANSI for PreWriteTest {
	fn write_ansi(&mut self, esc_seq: &EscSeq) {
	    let (esc,seq) = (esc_seq.esc, &esc_seq.seq[..]);
	    self.res += match esc {
		Esc::None => "<TXT:>",
		Esc::Esc => panic!("write_ansi called after only \\e"),
		Esc::CSI => "<CSI:>",
		Esc::NF => "<NF:>",
		Esc::STT => "<STT:>",
	    };
	    self.res += std::str::from_utf8(seq).expect("<invalid utf8>");
	}
    }
    impl io::Write for PreWriteTest {
	fn flush(&mut self) -> io::Result<()> {
	    if self.leftover.esc == Esc::None && self.leftover.seq.is_empty() {
		Ok(())
	    } else {
		Err(io::Error::new(io::ErrorKind::Other,"Incomplete control sequence cannot be flushed!"))
	    }
	}
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
	    let mut esc_seq = self.leftover.clone();
	    let l = self.pre_write(buf, &mut esc_seq);
	    self.leftover = esc_seq;
	    Ok(l)
	}
    }
}

#[derive(Debug)]
enum EscASCII {
    None,
    BEL,
    BS,
    HT,
    LF,
    CR,
    DEL,
}
trait WriteASCII {
    fn write_ascii(&mut self, esc: EscASCII);
}
impl<T: FnMut(Option<usize>)> WriteASCII for State<'_, T> {
    fn write_ascii(&mut self, esc: EscASCII) {
	match esc {
	    EscASCII::None | EscASCII::BEL | EscASCII::DEL => (),
	    EscASCII::BS => if self.col > 0 {
		self.col -= 1;
		self.del_trail();
		self.draw(Some(self.lines.cur()));
	    },
	    EscASCII::HT => {
		self.col = ((self.col>>3)+1)<<3;
		self.trail();
		self.draw(Some(self.lines.cur()));
	    },
	    EscASCII::LF => {
		self.draw(Some(self.lines.cur()));
		self.del_trail();
		let cur = self.lines.cur() +
		    if self.lines.cur() == self.lines.len()-1 {
			self.draw(None);
			self.lines.add(Vec::new())
		    } else { 1 };
		self.lines.set_cur(&mut self.draw, cur);
		self.col = 0;
		self.draw(Some(self.lines.cur()));
	    },
	    EscASCII::CR => {
		self.col = 0;
		self.del_trail();
		self.draw(Some(self.lines.cur()));
	    },
	};
    }
}

#[cfg(test)]
mod write_ascii_tests {
    use super::*;
    use crate::t::state_test::*;
    #[test]
    fn lf_draw() {
	let draw_text = draw_test(|state| {
	    state.write("a ".as_bytes());  // Some(0)
	    state.write_ascii(EscASCII::LF);  // None None
	    state.write_all("ab\x1b[A".as_bytes());  // Some(1) Some(1) Some(0)
	    state.write_ascii(EscASCII::LF);  // Some(0) Some(1)
	    state.write("b".as_bytes());  // Some(1)
	    assert_eq!(state.lines.cur(),1);
	    assert_eq!(state.col, 1);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"a");
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"bb");
	    state.write_ascii(EscASCII::LF);  // None None
	    state.write_ascii(EscASCII::LF);  // None None
	    state.write_ascii(EscASCII::LF);  // None None
	    state.write_ascii(EscASCII::LF);  // None Some(4) Some(4)
	    assert_eq!(state.lines.cur(),4);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"bb");
	});
	assert_eq!(draw_text, "<Some(0)><None><None><Some(1)><Some(1)><Some(0)><Some(0)><Some(1)><Some(1)>\
			       <None><None><None><None><None><None><None><Some(4)><Some(4)>");
    }
    #[test]
    fn cr_draw() {
	let draw_text = draw_test(|state| {
	    state.write("a ".as_bytes());
	    state.write_ascii(EscASCII::CR);
	    assert_eq!(state.col, 0);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"a");
	});
	assert_eq!(draw_text, "<Some(0)><Some(0)>");
    }
    #[test]
    fn ht_draw() {
	let draw_text = draw_test(|state| {
	    state.write("a ".as_bytes());
	    state.write_ascii(EscASCII::HT);
	    assert_eq!(state.col, 8);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"a       ");
	    state.write_ascii(EscASCII::HT);
	    assert_eq!(state.col,16);
	});
	assert_eq!(draw_text, "<Some(0)><Some(0)><Some(0)>");
    }
    #[test]
    fn bs_draw() {
	let draw_text = draw_test(|state| {
	    state.write("a ".as_bytes());
	    state.write_ascii(EscASCII::BS);
	    state.write_ascii(EscASCII::BS);
	    state.write_ascii(EscASCII::BS);
	    assert_eq!(state.col, 0);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"a");
	});
	assert_eq!(draw_text, "<Some(0)><Some(0)><Some(0)>");
    }
}

enum EscCMD {
    None,
    CUU,
    CUD,
    CUF,
    CUB,
    CNL,
    CPL,
    CHA,
    CUP,
    ED,
    EL,
    SU,
    SD,
    DSR,
    SGR,
    SET,
}
trait WriteCMD {
    fn write_cmd(&mut self, esc: EscCMD, n: Option<u16>, m: Option<u16>);
    fn write_txt(&mut self, txt: &[u8]);
}
impl<T: WriteCMD> WriteANSI for T {
    fn write_ansi(&mut self, esc_seq: & EscSeq) {
	match esc_seq.esc {
	    Esc::None => self.write_txt(&esc_seq.seq),
	    Esc::CSI => {
		println!("got CSI sequence: ESC {}",std::str::from_utf8(&esc_seq.seq).ok().unwrap());
		let args: Vec<Option<u16>> = esc_seq.seq.split(|b: &u8| !(*b).is_ascii_digit())
		    .map(|i: &[u8]| std::str::from_utf8(i)
			 .expect("ascii digits should be valid utf8")
			 .parse::<u16>().ok()  // only possible error should be for empty split
		    ).take(2).collect();
		self.write_cmd( match esc_seq.seq.last().expect(" CSI escape sequence should not be empty!") {
		    b'A' => EscCMD::CUU,
		    b'B' => EscCMD::CUD,
		    b'C' => EscCMD::CUF,
		    b'D' => EscCMD::CUB,
		    b'E' => EscCMD::CNL,
		    b'F' => EscCMD::CPL,
		    b'G' => EscCMD::CHA,
		    b'H' | b'f' => EscCMD::CUP,
		    b'J' => EscCMD::ED,
		    b'K' => EscCMD::EL,
		    b'S' => EscCMD::SU,
		    b'T' => EscCMD::SD,
		    b'n' if args[0]==Some(6) => EscCMD::DSR,
		    b'm' => EscCMD::SGR,
		    b'~' => EscCMD::SET,
		    _ => EscCMD::None,
		}, args[0], args[1]);
	    },
	    Esc::NF | Esc::STT => (),
	    Esc::Esc => panic!("read_ansi should not be called with Esc type Esc!"),
	}
    }
}

#[cfg(test)]
mod write_ansi_test {
    use super::*;
    struct WriteANSITest {
	res: String,
    }
    impl WriteCMD for WriteANSITest {
	fn write_cmd(&mut self, esc: EscCMD, n: Option<u16>, m: Option<u16>) {
	    self.res += match esc {
		EscCMD::None => "<None",
		EscCMD::CUU => "<CUU",
		EscCMD::CUD => "<CUD",
		EscCMD::CUF => "<CUF",
		EscCMD::CUB => "<CUB",
		EscCMD::CNL => "<CNL",
		EscCMD::CPL => "<CPL",
		EscCMD::CHA => "<CHA",
		EscCMD::CUP => "<CUP",
		EscCMD::ED => "<ED",
		EscCMD::EL => "<EL",
		EscCMD::SU => "<SU",
		EscCMD::SD => "<SD",
		EscCMD::DSR => "<DSR",
		EscCMD::SGR => "<SGR",
		EscCMD::SET => "<SET",
	    };
	    if let Some(n) = n {self.res += &format!("{}",n)[..];}
	    self.res += ";";
	    if let Some(m) = m {self.res += &format!("{}",m)[..];}
	    self.res += ">";
	}
	fn write_txt(&mut self, txt: &[u8]) {
	    self.res += std::str::from_utf8(txt).unwrap_or("invalid utf8");
	}
    }
    #[test]
    fn basic_text() {
	let mut test = WriteANSITest {res: String::new(),};
	test.write_ansi(&EscSeq {
	    esc: Esc::None,
	    seq: Vec::from("text".as_bytes()),
	    inv: false,
	});
	assert_eq!(test.res,"text");
    }
    impl EscSeq {
	fn set(&mut self,val: &str) -> &Self {
	    self.seq = Vec::from(val.as_bytes());
	    self
	}
    }
    #[test]
    fn csi() {
	let mut test = WriteANSITest {res: String::new(),};
	let mut esc = EscSeq {
	    esc: Esc::CSI,
	    seq: Vec::from("15A".as_bytes()),
	    inv: false,
	};
	test.write_ansi(&esc);
	test.write_ansi(esc.set("3;B"));
	test.write_ansi(esc.set("C"));
	test.write_ansi(esc.set("3;4H"));
	test.write_ansi(esc.set("f"));
	test.write_ansi(esc.set("6n"));
	test.write_ansi(esc.set("n"));
	test.write_ansi(esc.set("99999~"));
	assert_eq!(test.res,"<CUU15;><CUD3;><CUF;><CUP3;4><CUP;><DSR6;><None;><SET;>");
    }
}

#[cfg(test)]
mod write_cmd_tests {
    use super::*;
    use crate::t::state_test::*;
    
    #[test]
    fn cuu_draw() {
	let draw_text = draw_test(|state| {
	    state.write_cmd(EscCMD::CUU,Some(1),None,);  // Some(0) Some(0)
	    state.write_ascii(EscASCII::LF);  // None, None
	    state.write("hallo".as_bytes());  // Some(1)
	    state.write_cmd(EscCMD::CUU,None,None);  // None
	    state.write("hallo".as_bytes());  // Some(0)
	    assert_eq!(state.lines.cur(),0);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"     hallo");
	});
	assert_eq!(draw_text, "<Some(0)><Some(0)><None><None><Some(1)><None><Some(0)>");
    }
    #[test]
    fn cud_draw() {
	let draw_text = draw_test(|state| {
	    state.write_ascii(EscASCII::LF);  // None, None
	    state.write("hallo".as_bytes());  // Some(1)
	    state.write_cmd(EscCMD::CUU,None,None);  // None
	    state.write("hallo".as_bytes());  // Some(0)
	    state.write_cmd(EscCMD::CUD,Some(2),None);  // Some(0) Some(1)
	    assert_eq!(state.lines.cur(),1);
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"hallo     ");
	});
	assert_eq!(draw_text, "<None><None><Some(1)><None><Some(0)><Some(0)><Some(1)>");
    }
    #[test]
    fn cuf_cub_draw() {
	let draw_text = draw_test(|state| {
	    state.write("hallo".as_bytes());  // Some(0)
	    state.write_cmd(EscCMD::CUB,None,None);  // Some(0)
	    assert_eq!(state.col,4);
	    state.write("n".as_bytes());  // Some(0)
	    state.write_cmd(EscCMD::CUF,Some(2),None);  // Some(0)
	    assert_eq!(state.col,7);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"halln  ");
	});
	assert_eq!(draw_text, "<Some(0)><Some(0)><Some(0)><Some(0)>");
    }
    #[test]
    fn cnl_cpl_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("a\nb".as_bytes());  // Some(0) None None Some(1)
	    state.write_cmd(EscCMD::CPL,None,None,);  // Some(1) Some(0)
	    state.write("c".as_bytes());  // Some(0)
	    state.write_cmd(EscCMD::CNL,Some(2),None); // Some(0) Some(1)
	    assert_eq!(state.lines.cur(),1);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"c");
	});
	assert_eq!(draw_text, "<Some(0)><None><None><Some(1)><Some(1)><Some(0)><Some(0)><Some(0)><Some(1)>");
    }
    #[test]
    fn cha_cup_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("a\nb".as_bytes());  // Some(0) None None Some(1)
	    state.write_cmd(EscCMD::CHA,Some(5),None);  // Some(1)
	    state.write("a".as_bytes());  // Some(1)
	    state.write_cmd(EscCMD::CUP,Some(3),None);  // None
	    state.write("c".as_bytes());  // Some(2)
	    assert_eq!(state.lines.cur(),2);
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"b   a");
	    assert_eq!(String::from_utf8(state.lines[2].clone()).ok().unwrap_or(String::from("invalid utf8")),"c");
	    assert_eq!(state.lines.start,0);
	    assert_eq!(state.lines.end,3);
	});
	assert_eq!(draw_text, "<Some(0)><None><None><Some(1)><Some(1)><Some(1)><None><Some(2)>");
    }
    #[test]
    fn ed_0_1_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("hallo\n hallo\n  hallo\x1b[2;4H".as_bytes());
	    // <Some(0)><None><None><Some(1)><None><None><Some(2)><Some(2)><Some(1)>
	    state.write_cmd(EscCMD::ED,None,None);  // <None>
	    state.write("\x08".as_bytes());  // <Some(1)>
	    state.write_cmd(EscCMD::ED,Some(1),None);  // <None>
	    assert_eq!(state.lines.start,1);
	    assert_eq!(state.lines.cur(),1);
	    assert_eq!(state.lines.end,2);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"  a");
	    assert_eq!(String::from_utf8(state.lines[2].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(state.col,2);
	});
	assert_eq!(draw_text, "<Some(0)><None><None><Some(1)><None><None><Some(2)><Some(2)><Some(1)><None><Some(1)><None>");
    }
    #[test]
    fn ed_2_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("hallo\n hallo\n  hallo\x1b[2;4H".as_bytes());
	    // <Some(0)><None><None><Some(1)><None><None><Some(2)><Some(2)><Some(1)>
	    state.write_cmd(EscCMD::ED,Some(2),None);  // <None>
	    assert_eq!(state.lines.start,1);
	    assert_eq!(state.lines.cur(),1);
	    assert_eq!(state.lines.end,2);
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"   ");
	    assert_eq!(String::from_utf8(state.lines[2].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(state.col,3);
	});
	assert_eq!(draw_text, "<Some(0)><None><None><Some(1)><None><None><Some(2)><Some(2)><Some(1)><None>");
    }
    #[test]
    fn el_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("hallohallo\x08\x08".as_bytes());  // <Some(0)><Some(0)><Some(0)>
	    state.write_cmd(EscCMD::EL,None,None);  // <Some(0)>
	    state.write("\x08".as_bytes());  // <Some(0)>
	    state.write_cmd(EscCMD::EL,Some(1),None);  // <Some(0)>
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"       l");
	    assert_eq!(state.col,7);
	    state.write_cmd(EscCMD::EL,Some(2),None);  // <Some(0)>
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"       ");
	});
	assert_eq!(draw_text, "<Some(0)><Some(0)><Some(0)><Some(0)><Some(0)><Some(0)><Some(0)>");
    }
    #[test]
    fn su_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("a\n\n\nd".as_bytes());  // <Some(0)><None><None><None><None><None><None><Some(3)>
	    state.write_cmd(EscCMD::SU,Some(2),None);  // <None>
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"d");
	    assert_eq!(String::from_utf8(state.lines[2].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(String::from_utf8(state.lines[3].clone()).ok().unwrap_or(String::from("invalid utf8"))," ");
	    assert_eq!(state.lines.start,1);
	    assert_eq!(state.lines.cur(),3);
	    assert_eq!(state.lines.end,4);
	});
	assert_eq!(draw_text, "<Some(0)><None><None><None><None><None><None><Some(3)><None>");
    }
    #[test]
    fn sd_draw() {
	let draw_text = draw_test(|state| {
	    state.write_all("a\n\n\nd".as_bytes());  // <Some(0)><None><None><None><None><None><None><Some(3)>
	    state.write_cmd(EscCMD::SD,Some(2),None);  // <None>
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(String::from_utf8(state.lines[1].clone()).ok().unwrap_or(String::from("invalid utf8")),"");
	    assert_eq!(String::from_utf8(state.lines[2].clone()).ok().unwrap_or(String::from("invalid utf8")),"a");
	    assert_eq!(String::from_utf8(state.lines[3].clone()).ok().unwrap_or(String::from("invalid utf8"))," ");
	    assert_eq!(state.lines.start,2);
	    assert_eq!(state.lines.cur(),3);
	    assert_eq!(state.lines.end,4);
	});
	assert_eq!(draw_text, "<Some(0)><None><None><None><None><None><None><Some(3)><None>");
    }
    #[test]
    fn sgr() {
	draw_test(|state| {
	    state.write_all("hi \x1b[45m hi \x1b[49m hi".as_bytes());
	    assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"hi #hi# hi");
	});
    }
}

impl<T: FnMut(Option<usize>)> WriteCMD for State<'_, T> {
    fn write_cmd(&mut self, esc: EscCMD, n: Option<u16>, m: Option<u16>) {
	match esc {
	    EscCMD::None => (),
	    EscCMD::CUU => {
		self.draw(Some(self.lines.cur()));
		self.del_trail();
		self.lines.set_cur( &mut self.draw, self.lines.cur()
			-cmp::min(self.lines.cur(), n.unwrap_or(1).into()));
		self.trail();
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::CUD => {
		self.draw(Some(self.lines.cur()));
		self.del_trail();
		self.lines.set_cur( &mut self.draw, self.lines.cur()
			+cmp::min(self.lines.len()-self.lines.cur()-1,n.unwrap_or(1).into()));
		self.trail();
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::CUF => {
		self.col += <u16 as Into<usize>>::into(n.unwrap_or(1));
		self.draw(Some(self.lines.cur()));
		self.trail();
	    },
	    EscCMD::CUB => {
		self.col -= cmp::min(self.col, <u16 as Into<usize>>::into(n.unwrap_or(1)));
		self.draw(Some(self.lines.cur()));
		self.del_trail();
	    },
	    EscCMD::CNL => {
		self.draw(Some(self.lines.cur()));
		self.del_trail();
		self.lines.set_cur( &mut self.draw, self.lines.cur()
			+cmp::min(self.lines.len()-self.lines.cur()-1,n.unwrap_or(1).into()));
		self.col = 0;
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::CPL => {
		self.draw(Some(self.lines.cur()));
		self.del_trail();
		self.lines.set_cur( &mut self.draw, self.lines.cur()
			-cmp::min(self.lines.cur(), n.unwrap_or(1).into()));
		self.col = 0;
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::CHA => {
		self.col = one_to_zero(n);
		self.del_trail();
		self.trail();
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::CUP => {
		self.draw(Some(self.lines.cur()));
		self.del_trail();
		let n = cmp::min(one_to_zero(n), self.lines.max-1);
		while self.lines.len()<=n {
		    self.lines.add(Vec::new());
		    self.draw(None);
		}
		self.lines.set_cur( &mut self.draw, n);
		self.col = one_to_zero(m);
		self.trail();
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::ED => {
		let col = self.col;
		match n.unwrap_or(0) {
		    0 => {
			for i in self.lines.cur()+1..self.lines.len() {
			    self.lines[i].clear();
			}
			self.lines.get().truncate(col);
			self.lines.end = self.lines.cur()+1;
		    },
		    1 => {
			for i in 0..self.lines.cur() {
			    self.lines[i].clear();
			}
			self.lines.get().get_mut(0..col)
			    .expect("0..cur should be valid index on current line!")
			    .fill(b' ');
			self.lines.start = self.lines.cur();
		    },
		    2 => {
			for i in 0..self.lines.len() {
			    self.lines[i].clear();
			}
			self.trail();
			self.lines.start = self.lines.cur();
			self.lines.end = self.lines.cur()+1;
		    },
		    _ => (),
		};
		self.draw(None);
	    },
	    EscCMD::EL => {
		let col = self.col;
		match n.unwrap_or(0) {
		    0 => self.lines.get().truncate(col),
		    1 => self.lines.get().get_mut(..col)
			.expect("0..cur should be valid index for current line!")
			.fill(b' '),
		    2 => {
			self.lines.get().clear();
			self.trail();
		    },
		    _ => (),
		};
		self.draw(Some(self.lines.cur()));
	    },
	    EscCMD::SU => {
		let n = cmp::min(n.unwrap_or(1).into(),self.lines.len());
		self.del_trail();
		for i in 0..n {self.lines[i].clear();}
		self.lines.inc_ofs(n);
		self.lines.start -= cmp::min(self.lines.start,n);
		while self.lines[self.lines.start].is_empty() && self.lines.start < self.lines.cur() {
		    self.lines.start += 1;
		}
		self.trail();
		self.draw(None);
	    },
	    EscCMD::SD => {
		self.del_trail();
		let l = self.lines.len();
		let n = cmp::min(n.unwrap_or(1).into(),l);
		for i in l-n..l {self.lines[i].clear();}
		self.lines.dec_ofs(n);
		self.lines.start += cmp::min(n,self.lines.cur()-self.lines.start);
		self.lines.end = cmp::min(self.lines.end+n,self.lines.len());
		while self.lines[self.lines.end].is_empty() && self.lines.end > self.lines.cur()+1 {
		    self.lines.end -= 1
		};
		self.trail();
		self.draw(None);
	    },
	    EscCMD::DSR => {
		let _msg = format!("\x1b[{};{}R", self.lines.cur(), self.col);
	    },
	    EscCMD::SGR => {
		match n {
		    Some(40..=48) => self.leftover.inv = true,
		    Some(0) | Some(49) => self.leftover.inv = false,
		    _ => (),
		}
	    },
	    EscCMD::SET => {   // CSI_n_;73~ to set max height to _n_, default 5
		match m {
		    Some(73) => {
			let len: u16 = n.unwrap_or(5);
			if (len as usize) < self.lines.max {
			    self.lines = QueueView::<Vec<u8>>::new(len);
			    self.col=0;
			}
			self.draw(None);
		    },
		    _ => (),
		}
	    },
	}
    }
    fn write_txt(&mut self, txt: &[u8]) {
	let cp = &txt[..cmp::min( txt.len(), 0xffff-self.col )];
	let col = self.col;
	let l = self.lines.get();
	if col+cp.len() >= l.len() {
	    l.truncate(col);
	    l.extend_from_slice(cp);
	    
	} else {
	    let _ = &l[col..col+cp.len()].copy_from_slice(cp);
	}
	self.col += cp.len();
	self.draw(Some(self.lines.cur()));
    }
}
impl<T: FnMut(Option<usize>)> State<'_, T> {
    fn trail(&mut self) {
	let col = self.col;
	let l = self.lines.get();
	if l.len() < col { l.resize(col, b' ') };
    }
    fn del_trail(&mut self) {
	let l = self.lines.get();
	l.truncate(l.len()-l.iter().rev().take_while(|b| **b==b' ').count());
    }
}
#[cfg(test)]
mod trail {
    use super::*;
    use std::io::Write;
    #[test]
    fn test() {
	let mut state: State<'_,Box<dyn FnMut(Option<usize>)>> = State::new();
	state.write("a".as_bytes());
	state.col = 5;
	state.trail();
	assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"a    ");
	state.del_trail();
	assert_eq!(String::from_utf8(state.lines[0].clone()).ok().unwrap_or(String::from("invalid utf8")),"a");
    }
}
fn one_to_zero(x: Option<u16>) -> usize {
    match x {
	None | Some(0) => 0,
	Some(x) => <u16 as Into<usize>>::into(x)-1,
    }
}
