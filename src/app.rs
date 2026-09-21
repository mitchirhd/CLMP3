use std::{
    collections::HashMap,
    io,
    rc::Rc,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::{
    app_state::AppState,
    audio::{AudioControls, AudioPlayer},
    enums::CLMP3Event,
};
use crate::{enums::CLMP3Widget, widgets::main_widget::MainWidget};

fn handle_input_events(tx: Sender<CLMP3Event>) {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => {
                tx.send(CLMP3Event::KeyInput(key_event)).unwrap()
            }
            // crossterm::event::Event::Mouse(mouse_event) => {}
            crossterm::event::Event::Resize(_, _) => tx.send(CLMP3Event::Redraw).unwrap(),
            _ => {}
        }
    }
}

fn redraw_loop(tx: Sender<CLMP3Event>) {
    loop {
        tx.send(CLMP3Event::Redraw).unwrap();
        thread::sleep(Duration::from_secs_f64(1.0 / 10.0));
    }
}

pub struct App {
    exit: bool,
    event_tx: Sender<CLMP3Event>,
    event_rx: Receiver<CLMP3Event>,
    main_widget: MainWidget,
    widget_state: AppState,
    handler_map: HashMap<(KeyEventKind, KeyModifiers, KeyCode), Rc<dyn Fn(&mut App)>>,
    audio_player: AudioPlayer,
    audio_controls: Arc<AudioControls>,
}

impl App {
    pub fn new() -> App {
        let (event_tx, event_rx) = mpsc::channel::<CLMP3Event>();
        let (audio_player, audio_controls, queue) = AudioPlayer::new(event_tx.clone());
        let widget_state = AppState::new();

        audio_controls.set_volume(widget_state.volume_state.value() as f32);

        App {
            exit: false,
            event_tx: event_tx,
            event_rx: event_rx,
            main_widget: MainWidget::new(audio_controls.clone(), queue),
            widget_state: widget_state,
            handler_map: App::get_handler_map(),
            audio_player,
            audio_controls,
        }
    }

    fn get_handler_map() -> HashMap<(KeyEventKind, KeyModifiers, KeyCode), Rc<dyn Fn(&mut App)>> {
        let mut map: HashMap<(KeyEventKind, KeyModifiers, KeyCode), Rc<dyn Fn(&mut App)>> =
            HashMap::new();

        map.insert(
            (
                KeyEventKind::Press,
                KeyModifiers::CONTROL,
                KeyCode::Char('c'),
            ),
            Rc::new(App::exit_app),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('p')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widget::Player)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('d')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widget::Display)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('l')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widget::Load)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('q')),
            Rc::new(|app| App::set_selected_widget(app, CLMP3Widget::Queue)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char(' ')),
            Rc::new(App::start_stop),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Up),
            Rc::new(|app| App::on_navigation_key(app, KeyCode::Up)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Down),
            Rc::new(|app| App::on_navigation_key(app, KeyCode::Down)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Left),
            Rc::new(|app| App::on_navigation_key(app, KeyCode::Left)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Right),
            Rc::new(|app| App::on_navigation_key(app, KeyCode::Right)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Enter),
            Rc::new(|app| App::on_navigation_key(app, KeyCode::Enter)),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Backspace),
            Rc::new(|app| App::on_navigation_key(app, KeyCode::Backspace)),
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

        self.main_widget.draw(terminal, &mut self.widget_state)?;

        self.run(terminal)
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            let event = self
                .event_rx
                .recv()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            match event {
                CLMP3Event::KeyInput(key_event) => self.handle_key_event(key_event),
                // CLMP3Event::MouseInput(mouse_event) => todo!(),
                CLMP3Event::Redraw => {}
                CLMP3Event::SongEnded => self.handle_song_ended(),
            }
            self.main_widget.draw(terminal, &mut self.widget_state)?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self
            .handler_map
            .get(&(key_event.kind, key_event.modifiers, key_event.code))
            .cloned()
        {
            Some(func) => func(self),
            None => (),
        }
    }

    fn handle_song_ended(&mut self) {
        if self.audio_player.has_next() {
            // falls was in queue
            self.audio_player.play_next();
        } else {
            self.audio_controls.set_is_stopped(true);
        }
    }

    fn exit_app(app: &mut App) {
        app.exit = true;
    }

    fn set_selected_widget(app: &mut App, val: CLMP3Widget) {
        app.widget_state.set_selected_widget(val);
    }

    fn on_navigation_key(app: &mut App, key: KeyCode) {
        app.main_widget
            .on_navigation_key(key, &mut app.widget_state);
    }

    fn start_stop(app: &mut App) {
        if app.audio_player.has_next() || !app.audio_controls.get_is_stopped() {
            // falls was in queue, oder aktuell am spielen
            app.audio_player.start_stop();
        } else {
            app.widget_state.set_selected_widget(CLMP3Widget::Load);
        }
    }
}
