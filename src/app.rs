use std::{
    collections::HashMap,
    io,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::{app_state::AppState, audio::AudioPlayer, enums::Clmp3Event};
use crate::{enums::Clmp3Widget, widgets::main_widget::MainWidget};

const NAV_MODIFIERS: [KeyModifiers; 3] = [
    KeyModifiers::NONE,
    KeyModifiers::CONTROL,
    KeyModifiers::SHIFT,
];

const NAV_KEYS: [KeyCode; 11] = [
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Enter,
    KeyCode::Backspace,
    KeyCode::Delete,
    KeyCode::End,
    KeyCode::Home,
    KeyCode::PageUp,
    KeyCode::PageDown,
];

fn handle_input_events(tx: Sender<Clmp3Event>) {
    while let Ok(event) = crossterm::event::read() {
        let msg = match event {
            Event::Key(key) => Clmp3Event::KeyInput(key),
            Event::Resize(_, _) => Clmp3Event::Redraw,
            _ => continue,
        };
        if tx.send(msg).is_err() {
            break;
        }
    }
}

fn redraw_loop(tx: Sender<Clmp3Event>) {
    while tx.send(Clmp3Event::Redraw).is_ok() {
        thread::sleep(Duration::from_millis(100));
    }
}

type HandlerMap = HashMap<(KeyEventKind, KeyModifiers, KeyCode), fn(&mut App)>;

pub struct App {
    exit: bool,
    event_tx: Sender<Clmp3Event>,
    event_rx: Receiver<Clmp3Event>,
    main_widget: MainWidget,
    app_state: AppState,
    handler_map: HandlerMap,
    audio_player: AudioPlayer,
}

impl App {
    pub fn new() -> App {
        let (event_tx, event_rx) = mpsc::channel::<Clmp3Event>();
        let (audio_player, audio_controls, queue) = AudioPlayer::new(event_tx.clone());
        let app_state = AppState::new(audio_controls, queue);

        App {
            exit: false,
            event_tx,
            event_rx,
            main_widget: MainWidget::new(),
            app_state,
            handler_map: App::get_handler_map(),
            audio_player,
        }
    }

    fn get_handler_map() -> HandlerMap {
        let mut map: HandlerMap = HashMap::new();

        map.insert(
            (
                KeyEventKind::Press,
                KeyModifiers::CONTROL,
                KeyCode::Char('c'),
            ),
            App::exit_app,
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('p')),
            |app| app.app_state.set_selected_widget(Clmp3Widget::Player),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('d')),
            |app| app.app_state.set_selected_widget(Clmp3Widget::Display),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('l')),
            |app| app.app_state.set_selected_widget(Clmp3Widget::Load),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('q')),
            |app| app.app_state.set_selected_widget(Clmp3Widget::Queue),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char(' ')),
            App::start_stop,
        );

        map
    }

    pub fn start(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let event_tx = self.event_tx.clone();
        thread::spawn(move || {
            handle_input_events(event_tx);
        });

        let event_tx = self.event_tx.clone();
        thread::spawn(move || {
            redraw_loop(event_tx);
        });

        self.main_widget.draw(terminal, &mut self.app_state)?;

        self.run(terminal)
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            let event = self.event_rx.recv().map_err(io::Error::other)?;

            match event {
                Clmp3Event::KeyInput(key_event) => self.handle_key_event(key_event),
                // Clmp3Event::MouseInput(mouse_event) => todo!(),
                Clmp3Event::Redraw => {}
                Clmp3Event::SongEnded => self.handle_song_ended(),
            }
            self.main_widget.draw(terminal, &mut self.app_state)?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        let KeyEvent {
            kind,
            modifiers,
            code,
            ..
        } = key_event;
        if kind != KeyEventKind::Press {
            return;
        }

        if let Some(handler) = self.handler_map.get(&(kind, modifiers, code)).copied() {
            handler(self);
        } else if NAV_KEYS.contains(&code) && NAV_MODIFIERS.contains(&modifiers) {
            self.main_widget
                .on_navigation_key(modifiers, code, &mut self.app_state);
        }
    }

    fn handle_song_ended(&mut self) {
        let state = &mut self.app_state;
        if state.audio_controls.previous_requested() {
            state.queue.replay();
        } else {
            state.queue.current_to_previous();
        }

        state.audio_controls.reset();
        if self.audio_player.has_next() {
            self.audio_player.play_next(state);
        } else {
            state.audio_controls.set_is_stopped(true);
            state.queue_state.select(None);
        }
    }

    fn exit_app(app: &mut App) {
        app.exit = true;
    }

    fn start_stop(app: &mut App) {
        if app.audio_player.has_next() || !app.app_state.audio_controls.get_is_stopped() {
            app.audio_player.start_stop(&mut app.app_state);
        } else {
            app.app_state.set_selected_widget(Clmp3Widget::Load);
        }
    }
}
