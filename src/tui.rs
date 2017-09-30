use std::io::{self, stdout, Stdout};
use std::collections::HashMap;

use futures::{future, Async, Future, Poll, Sink, Stream};
use tokio_core::reactor::Handle;

use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};

use termion::terminal_size;
use termion::input::MouseTerminal;
use termion::event::{Event, Key, MouseButton, MouseEvent};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;

use xrl::{Client, ClientResult, Frontend, FrontendBuilder, ScrollTo, ServerResult, Style, Update};

use errors::*;
use input::Input;
use view::View;

pub struct Tui {
    pub pending_open_requests: Vec<ClientResult<String>>,
    pub delayed_events: Vec<CoreEvent>,
    pub views: HashMap<String, View>,
    pub current_view: String,
    pub events: UnboundedReceiver<CoreEvent>,
    pub handle: Handle,
    pub input: Input,
    pub client: Client,
    pub term_size: (u16, u16),
    pub term: MouseTerminal<AlternateScreen<RawTerminal<Stdout>>>,
    pub shutdown: bool,
}

impl Tui {
    pub fn new(handle: Handle, client: Client, events: UnboundedReceiver<CoreEvent>) -> Self {
        let term = stdout().into_raw_mode().unwrap();
        let mut input = Input::new();
        input.run();
        Tui {
            events: events,
            delayed_events: Vec::new(),
            pending_open_requests: Vec::new(),
            handle: handle,
            term_size: terminal_size().unwrap(),
            term: MouseTerminal::from(AlternateScreen::from(term)),
            views: HashMap::new(),
            current_view: "".into(),
            client: client,
            input: input,
            shutdown: false,
        }
    }

    pub fn handle_core_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::Update(update) => self.handle_update(update),
            CoreEvent::SetStyle(style) => self.handle_set_style(style),
            CoreEvent::ScrollTo(scroll_to) => self.handle_scroll_to(scroll_to),
        }
    }

    pub fn handle_update(&mut self, update: Update) {
        let Tui {
            ref mut views,
            ref mut delayed_events,
            ..
        } = *self;
        match views.get_mut(&update.view_id) {
            Some(view) => view.update_cache(update),
            None => delayed_events.push(CoreEvent::Update(update)),
        }
    }

    pub fn handle_scroll_to(&mut self, scroll_to: ScrollTo) {
        let Tui {
            ref mut views,
            ref mut delayed_events,
            ..
        } = *self;
        match views.get_mut(&scroll_to.view_id) {
            Some(view) => view.set_cursor(scroll_to.line, scroll_to.column),
            None => delayed_events.push(CoreEvent::ScrollTo(scroll_to)),
        }
    }

    pub fn handle_set_style(&mut self, style: Style) {
        // let view = self.views.get_mut(&style).unwrap();
        // view.set_style(style);
    }

    pub fn resize(&mut self) {
        let Tui {
            ref mut term_size,
            ref current_view,
            ref mut views,
            ..
        } = *self;
        let mut new_size = terminal_size()
            .chain_err(|| ErrorKind::TerminalSizeError)
            .unwrap();
        if new_size != *term_size {
            *term_size = new_size;
        }
        match views.get_mut(current_view) {
            Some(view) => view.resize(term_size.1),
            None => {}
        }
    }

    pub fn get_view_mut(&mut self, view_id: String) -> Result<&mut View> {
        Ok(self.views
            .get_mut(&view_id)
            .ok_or_else(|| ErrorKind::ViewNotFound)?)
    }

    pub fn insert(&mut self, character: char) {
        let future = self.client
            .char(&self.current_view, character)
            .map_err(|_| ());
        self.handle.spawn(future);
    }

    fn down(&mut self) {
        let future = self.client.down(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    fn up(&mut self) {
        let future = self.client.up(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    fn left(&mut self) {
        let future = self.client.left(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    fn right(&mut self) {
        let future = self.client.right(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    fn page_down(&mut self) {
        let future = self.client.page_down(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    fn page_up(&mut self) {
        let future = self.client.page_up(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    fn delete(&mut self) {
        let future = self.client.del(&self.current_view).map_err(|_| ());
        self.handle.spawn(future);
    }

    pub fn open(&mut self, file_path: &str) {
        let task = self.client.new_view(Some(file_path.to_string()));
        self.pending_open_requests.push(task);
    }

    pub fn exit(&mut self) {
        self.shutdown = true;
    }

    pub fn save(&mut self) {
        unimplemented!()
    }

    pub fn handle_input(&mut self, event: Event) {
        info!("handling input {:?}", event);
        match event {
            Event::Key(key) => match key {
                Key::Char(c) => self.insert(c),
                Key::Ctrl(c) => match c {
                    'c' => self.exit(),
                    'w' => self.save(),
                    _ => panic!("unexpected input"),
                },
                Key::Backspace => self.delete(),
                Key::Left => self.left(),
                Key::Right => self.right(),
                Key::Up => self.up(),
                Key::Down => self.down(),
                Key::PageUp => self.page_up(),
                Key::PageDown => self.page_down(),
                _ => panic!("unsupported key event"),
            },
            _ => panic!("unsupported event"),
        }
    }

    pub fn process_open_requests(&mut self) {
        let Tui {
            ref mut pending_open_requests,
            ref mut views,
            ref mut current_view,
            ..
        } = *self;

        let mut done = vec![];
        for (idx, task) in pending_open_requests.iter_mut().enumerate() {
            match task.poll() {
                Ok(Async::Ready(view_id)) => {
                    done.push(idx);
                    views.insert(view_id.clone(), View::new());
                    *current_view = view_id;
                }
                Ok(Async::NotReady) => continue,
                Err(e) => panic!("\"open\" task failed: {}", e),
            }
        }
        for idx in done.iter().rev() {
            pending_open_requests.remove(*idx);
        }
    }

    pub fn process_input(&mut self) {
        loop {
            match self.input.poll() {
                Ok(Async::Ready(Some(event))) => {
                    info!("got event {:?}", event);
                    self.handle_input(event);
                }
                // no more input
                Ok(Async::Ready(None)) => panic!("No more input"),
                Ok(Async::NotReady) => break,
                Err(_) => panic!("error polling input"),
            }
        }
    }

    pub fn process_core_events(&mut self) {
        loop {
            match self.events.poll() {
                Ok(Async::Ready(Some(event))) => {
                    info!("Handling event from core: {:?}", event);
                    self.handle_core_event(event);
                }
                // no more input
                Ok(Async::Ready(None)) => panic!("No more events"),
                Ok(Async::NotReady) => break,
                Err(_) => panic!("error polling input"),
            }
        }
    }

    pub fn process_delayed_events(&mut self) {
        let mut delayed_events: Vec<CoreEvent> = self.delayed_events.drain(..).collect();
        for event in delayed_events {
            self.handle_core_event(event);
        }
    }

    pub fn render(&mut self) {
        let Tui {
            ref mut views,
            ref mut term,
            ref current_view,
            ..
        } = *self;
        match views.get_mut(current_view) {
            Some(view) => view.render(term).unwrap(),
            None => {}
        }
    }
}

#[derive(Debug)]
pub enum CoreEvent {
    Update(Update),
    ScrollTo(ScrollTo),
    SetStyle(Style),
}

impl Future for Tui {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.process_open_requests();
        self.process_delayed_events();
        self.process_input();
        self.process_core_events();
        self.resize();
        self.render();

        if self.shutdown {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub struct TuiService(UnboundedSender<CoreEvent>);

impl Frontend for TuiService {
    fn update(&mut self, update: Update) -> ServerResult<()> {
        self.0.start_send(CoreEvent::Update(update));
        self.0.poll_complete();
        Box::new(future::ok(()))
    }

    fn scroll_to(&mut self, scroll_to: ScrollTo) -> ServerResult<()> {
        self.0.start_send(CoreEvent::ScrollTo(scroll_to));
        self.0.poll_complete();
        Box::new(future::ok(()))
    }

    fn set_style(&mut self, style: Style) -> ServerResult<()> {
        self.0.start_send(CoreEvent::SetStyle(style));
        self.0.poll_complete();
        Box::new(future::ok(()))
    }
}

pub struct TuiServiceBuilder(UnboundedSender<CoreEvent>);

impl TuiServiceBuilder {
    pub fn new() -> (Self, UnboundedReceiver<CoreEvent>) {
        let (tx, rx) = unbounded();
        (TuiServiceBuilder(tx), rx)
    }
}

impl FrontendBuilder<TuiService> for TuiServiceBuilder {
    fn build(self, _client: Client) -> TuiService {
        TuiService(self.0)
    }
}
