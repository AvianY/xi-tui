use std::io::{self, stdin};
use std::thread;

use futures::{Future, Poll, Sink, Stream};
use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};

use termion::event::Event;
// use termion::event::Key;
// use termion::event::MouseButton;
// use termion::event::MouseEvent;
use termion::input::TermRead;

pub struct Input {
    tx: UnboundedSender<Event>,
    rx: UnboundedReceiver<Event>,
}

impl Input {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Input { tx: tx, rx: rx }
    }

    pub fn run(&mut self) {
        let mut tx = self.tx.clone();

        thread::spawn(move || {
            info!("waiting for input events");
            for event_res in stdin().events() {
                match event_res {
                    // TODO: at least log the errors
                    Ok(event) => {
                        let _ = tx.start_send(event).unwrap();
                        let _ = tx.poll_complete().unwrap();
                    }
                    Err(e) => error!("{}", e),
                }
            }
            info!("stop waiting for input events");
        });
    }
}

impl Stream for Input {
    type Item = Event;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.rx.poll()
    }
}
//pub fn handle_event(&mut self, event: &Event) -> Result<()> {
//    match *event {
//        Event::Key(key) => match key {
//            Key::Char(c) => core.char(c)?,
//            Key::Ctrl(c) => match c {
//                'c' => {
//                    info!("received ^C: exiting");
//                    std::process::exit(0);
//                }
//                'w' => {
//                    info!("received ^W: writing current file");
//                    core.save()?;
//                }
//                _ => {
//                    bail!(ErrorKind::InputError);
//                }
//            },
//            Key::Backspace => {
//                core.del()?;
//            }
//            Key::Left => {
//                core.left()?;
//            }
//            Key::Right => {
//                core.right()?;
//            }
//            Key::Up => {
//                core.up()?;
//            }
//            Key::Down => {
//                core.down()?;
//            }
//            Key::PageUp => {
//                core.page_up()?;
//            }
//            Key::PageDown => {
//                core.page_down()?;
//            }
//            _ => {
//                error!("unsupported key event");
//                bail!(ErrorKind::InputError);
//            }
//        },
//        Event::Mouse(mouse_event) => match mouse_event {
//            MouseEvent::Press(press_event, y, x) => match press_event {
//                MouseButton::Left => {
//                    core.click(u64::from(x) - 1, u64::from(y) - 1)?;
//                }
//                MouseButton::WheelUp => {
//                    core.up()?;
//                }
//                MouseButton::WheelDown => {
//                    core.down()?;
//                }
//                _ => {}
//            },
//            MouseEvent::Release(..) => {}
//            MouseEvent::Hold(y, x) => {
//                core.drag(u64::from(x) - 1, u64::from(y) - 1)?;
//            }
//        },
//        _ => {
//            error!("unsupported event");
//            bail!(ErrorKind::InputError);
//        }
//    }
//    Ok(())
//}
