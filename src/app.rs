use std::{
    collections::HashMap,
    io,
    sync::mpsc::{self, Receiver},
    thread,
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::events::CLMP3Event;
use crate::widgets::main_widget::MainWidget;

fn handle_input_events(tx: mpsc::Sender<CLMP3Event>) {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => {
                tx.send(CLMP3Event::KeyInput(key_event)).unwrap()
            }
            crossterm::event::Event::Mouse(mouse_event) => {}
            crossterm::event::Event::Resize(_, _) => tx.send(CLMP3Event::Resize).unwrap(),
            _ => {}
        }
    }
}

pub struct App {
    exit: bool,
    main_widget: MainWidget,
    handler_map: HashMap<(KeyEventKind, KeyModifiers, KeyCode), fn(&mut App) -> io::Result<()>>,
}

impl App {
    pub fn new() -> App {
        App {
            exit: false,
            main_widget: MainWidget::new(),
            handler_map: App::get_handler_map(),
        }
    }

    fn get_handler_map()
    -> HashMap<(KeyEventKind, KeyModifiers, KeyCode), fn(&mut App) -> io::Result<()>> {
        let mut map: HashMap<
            (KeyEventKind, KeyModifiers, KeyCode),
            fn(&mut App) -> io::Result<()>,
        > = HashMap::new();

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('q')),
            |app: &mut App| {
                app.exit = true;
                Ok(())
            },
        );

        map
    }

    pub fn start(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (event_tx, event_rx) = mpsc::channel::<CLMP3Event>();

        thread::spawn(move || {
            handle_input_events(event_tx.clone());
        });

        self.draw(terminal)?;

        self.run(terminal, event_rx)
    }

    fn run(&mut self, terminal: &mut DefaultTerminal, rx: Receiver<CLMP3Event>) -> io::Result<()> {
        while !self.exit {
            let event = rx
                .recv()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            match event {
                CLMP3Event::KeyInput(key_event) => self.handle_key_event(key_event)?,
                CLMP3Event::MouseInput(mouse_event) => todo!(),
                CLMP3Event::Resize => {}
            }
            self.draw(terminal)?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
        match self
            .handler_map
            .get(&(key_event.kind, key_event.modifiers, key_event.code))
        {
            Some(func) => func(self),
            None => Ok(()),
        }
    }

    fn draw(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| frame.render_widget(&self.main_widget, frame.area()))?;
        Ok(())
    }
}
