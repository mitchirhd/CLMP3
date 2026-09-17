use std::{
    collections::HashMap,
    io,
    rc::Rc,
    sync::mpsc::{self, Receiver},
    thread,
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::{audio::AudioPlayer, enums::CLMP3Event};
use crate::{enums::CLMP3Widgets, widgets::main_widget::MainWidget};

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
    handler_map:
        HashMap<(KeyEventKind, KeyModifiers, KeyCode), Rc<dyn Fn(&mut App) -> io::Result<()>>>,
    audio_player: AudioPlayer,
}

impl App {
    pub fn new() -> App {
        let (audio_player, player_controls) = AudioPlayer::new();
        App {
            exit: false,
            main_widget: MainWidget::new(player_controls),
            handler_map: App::get_handler_map(),
            audio_player,
        }
    }

    fn get_handler_map()
    -> HashMap<(KeyEventKind, KeyModifiers, KeyCode), Rc<dyn Fn(&mut App) -> io::Result<()>>> {
        let mut map: HashMap<
            (KeyEventKind, KeyModifiers, KeyCode),
            Rc<dyn Fn(&mut App) -> io::Result<()>>,
        > = HashMap::new();

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('q')),
            Rc::new(App::exit_app),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('p')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widgets::Player)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('d')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widgets::Display)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('l')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widgets::Load)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('q')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widgets::Queue)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char(' ')),
            Rc::new(App::start_stop),
        );

        map
    }

    pub fn start(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (event_tx, event_rx) = mpsc::channel::<CLMP3Event>();

        thread::spawn(move || {
            handle_input_events(event_tx.clone());
        });

        self.main_widget.draw(terminal)?;

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
            self.main_widget.draw(terminal)?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
        match self
            .handler_map
            .get(&(key_event.kind, key_event.modifiers, key_event.code))
            .cloned()
        {
            Some(func) => func(self),
            None => Ok(()),
        }
    }

    fn exit_app(app: &mut App) -> io::Result<()> {
        app.exit = true;
        Ok(())
    }

    fn set_selected_widget(app: &mut App, val: CLMP3Widgets) -> io::Result<()> {
        app.main_widget.set_selected_widget(val);
        Ok(())
    }

    fn start_stop(app: &mut App) -> io::Result<()> {
        if app.audio_player.has_source {
            app.audio_player.start_stop();
        } else {
            app.main_widget.set_is_loading(true);
        }
        Ok(())
    }
}
