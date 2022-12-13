use std::cmp;
use std::iter::once;
use std::io::{Read,Result};
use super::State;

pub(super) struct QueueView<T: Empty>{
    vec: Vec<T>,
    ofs: usize,
    pub max: usize,
    pub start: usize,
    pub end: usize,
    cur: usize,
}
impl QueueView<Vec<u8>> {
    pub(super) fn new(n_lines: u16) -> QueueView<Vec<u8>> {
	QueueView {
	    vec: vec![Vec::new()],
	    ofs: 0,
	    max: n_lines.into(),
	    start: 0,
	    end: 1,
	    cur: 0,
	}
    }
}
impl<T: Empty> std::ops::Index<usize> for QueueView<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
	&self.vec[(i+self.ofs)%self.len()]
    }
}
impl<T: Empty> std::ops::IndexMut<usize> for QueueView<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
	let l = self.len();
	&mut self.vec[(i+self.ofs)%l]
    }
}
impl<T: Empty> QueueView<T> {
    /*pub(super) fn iter<'a>(&'a self) ->
    std::iter::Map<std::ops::Range<usize>, Box<dyn FnMut(usize) -> (usize, &'a T) + 'a>> {
	(self.start..self.end).into_iter().map(Box::new(|b| (b, &self[b])))
}*/
    pub fn update_max_lines(&mut self, config: &Config) {
	self.max = config.n_lines.into();
    }
    pub(super) fn set_cur<D: FnMut(Option<usize>)>(&mut self,draw: &mut Option<D>, cur: usize) {
	let old_cur = self.cur;
	self.cur = cmp::min( cur, self.vec.len() );
	let old = (self.start, self.end);
	if self.cur >= self.end { self.end = self.cur+1 };
	if self.cur < self.start { self.start = self.cur };
	while self.start<self.cur && self[self.start].is_empty() {self.start += 1};
	while self.cur<self.end-1 && self[self.end-1].is_empty() {self.end -= 1};
	if let Some(draw) = draw {
	    if old != (self.start, self.end) {
		draw(None);
	    } else {
		draw(Some(old_cur));
		draw(Some(self.cur));
	    }
	}
    }
    pub(super) fn inc_ofs(&mut self, n: usize) {
	self.ofs = (self.ofs+n)%self.len();
    }
    pub(super) fn dec_ofs(&mut self, n: usize) {
	self.ofs = (self.ofs+self.len()-n)%self.len();
    }
    pub(super) fn add(&mut self, t: T) -> usize {
	if self.vec.len() < self.max {
	    if self.ofs == 0 {
		self.vec.push(t);  // alternative always works but this is simpler if applicable
	    } else {
		self.vec.insert(self.ofs,t);
		self.ofs += 1;
	    }
	    1
	} else {
	    self.vec[self.ofs] = t;
	    self.inc_ofs(1);
	    0
	}
    }
    pub(super) fn len(&self) -> usize {
	self.vec.len()
    }
    pub(super) fn vis_len(&self) -> usize {
	self.end-self.start
    }
    pub(super) fn get(&mut self) -> &mut T {
	let cur = self.cur;
	&mut self[cur]
    }
    pub(super) fn cur(&self) -> usize {
	self.cur
    }
}
pub(super) trait Empty {
    fn is_empty(&self) -> bool;
}
impl<T> Empty for Vec<T> {
    fn is_empty(&self) -> bool {
	self.is_empty()
    }
}






pub struct Reader<'a,'c, T: FnMut(Option<usize>)> {
    state: &'a mut State<'c, T>,
    iter: Box<dyn Iterator<Item=Line>>,
    current: Option<Line>,
    col: usize,
}
impl<'a,'c, T: FnMut(Option<usize>)> From<&'a mut State<'c, T>> for Reader<'a,'c, T> {
    fn from(state: &'a mut State<'c, T>) -> Reader<'a,'c, T> {
	let iter = (0..state.lines.end).map(|u| Line::Line(u)).chain(once(
	    Line::Ctrl(format!("\x1b[{};{}H", state.lines.cur()+1, state.col+1).as_bytes().to_vec())));
	Reader {
	    current: Some(Line::Ctrl(format!("\x1b[{};73~",state.lines.max).as_bytes().to_vec())),
	    iter: Box::new(iter),
	    state: state,
	    col: 0,
	}
    }
}
impl<'a,'c, T: FnMut(Option<usize>)> Into<&'a mut State<'c, T>> for Reader<'a,'c, T> {
    fn into(self) -> &'a mut State<'c, T> {
	self.state
    }
}
impl<T: FnMut(Option<usize>)> Read for Reader<'_,'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
	if let Some(c) = &self.current {
	    let (line, end) = match c {
		Line::Line(line) => (&(*self.state.lines[*line])[..],
				     if *line == self.state.lines.len()-1 {None} else {Some(b'\n')}),
		Line::Ctrl(ctrl) => (&ctrl[..],None),
	    };
	    let len = line.len() - self.col;
	    if buf.len() >= len + end.is_some() as usize {
		buf[0..len].copy_from_slice(&line[self.col..]);
		if let Some(e) = end {buf[len] = e};
		self.current = self.iter.next();
		self.col = 0;
		Ok( len + end.is_some() as usize )
	    } else {
		buf.copy_from_slice(&line[self.col..self.col+buf.len()]);
		self.col += buf.len();
		Ok(buf.len())
	    }
	} else { Ok(0) }
    }
}
enum Line {
    Line(usize),
    Ctrl(Vec<u8>),
}
#[cfg(test)]
mod reader_tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn test() {
	let mut state: State<Box<dyn FnMut(Option<usize>)>> = State::new();
	state.write_all("a\nb\n\nc".as_bytes());
	let mut reader: Reader<Box<dyn FnMut(Option<usize>)>> = (&mut state).into();
	let mut buf: Vec<u8> = Vec::new();
	let n = reader.read_to_end(&mut buf).expect("read errror");
	assert_eq!(String::from_utf8(buf).ok().unwrap_or(String::from("invalid utf8")),"\x1b[5;73~a\nb\n\nc\x1b[4;2H");
	drop(reader);
	state.write("d".as_bytes());
    }
}





pub struct WindowPosition {
    pub right: bool,
    pub bottom: bool,
    pub x_offset: u16,
    pub y_offset: u16,
}
pub struct Config {
    pub dark: bool,
    pub line_width: u16,
    pub n_lines: u16,
    pub pos: WindowPosition,
    pub last_line_border: bool,
}
impl Config {
    pub fn new() -> Config {
	let pos = WindowPosition {
	    right: false,
	    bottom: true,
	    x_offset: 0,
	    y_offset: 44,
	};
	Config {
	    dark: false,
	    line_width: 50,
	    n_lines: 24,
	    pos: pos,
	    last_line_border: true,
	}
    }
}
