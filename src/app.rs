use std::{
    collections::HashMap,
    io,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::DynamicImage;
use ratatui::DefaultTerminal;

use crate::{
    app_state::AppState,
    audio::AudioPlayer,
    enums::{Clmp3Error, Clmp3Event},
    source_builder::SourceType,
};
use crate::{enums::Clmp3Widget, widgets::main_widget::MainWidget};

const MIN_TERMINAL_HEIGHT: u16 = 20;
const MIN_TERMINAL_WIDTH: u16 = 80;

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
}

impl App {
    pub fn new() -> anyhow::Result<App> {
        let (event_tx, event_rx) = mpsc::channel::<Clmp3Event>();
        let player = AudioPlayer::new(event_tx.clone())?;
        let app_state = AppState::new(event_tx.clone(), player);

        Ok(App {
            exit: false,
            event_tx,
            event_rx,
            main_widget: MainWidget::new()?,
            app_state,
            handler_map: App::handler_map(),
        })
    }

    fn handler_map() -> HandlerMap {
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
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('q')),
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
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('u')),
            |app| app.app_state.set_selected_widget(Clmp3Widget::Queue),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char('o')),
            |app| app.app_state.set_selected_widget(Clmp3Widget::Online),
        );

        map.insert(
            (KeyEventKind::Press, KeyModifiers::NONE, KeyCode::Char(' ')),
            App::on_space,
        );

        map
    }

    pub fn start(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
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

    fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.exit {
            let event = self.event_rx.recv().map_err(io::Error::other)?;

            let result = match event {
                Clmp3Event::KeyInput(key_event) => self.handle_key_event(key_event),
                Clmp3Event::Redraw => self.app_state.player.update(),
                Clmp3Event::NewSongStarted => self.load_current_pic(),
                Clmp3Event::CoverLoaded(source, dynamic_image) => {
                    self.on_cover_loaded(source, dynamic_image)
                }
            };

            if let Err(e) = result {
                self.app_state.last_error = Some((Clmp3Error::AudioError, vec![format!("{e:#}")]));
            }

            self.check_size(terminal)?;
            self.main_widget.draw(terminal, &mut self.app_state)?;
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        self.app_state.set_last_input_time(Instant::now());
        let KeyEvent {
            kind,
            modifiers,
            code,
            ..
        } = key_event;
        if kind != KeyEventKind::Press {
            return anyhow::Ok(());
        }

        if !self.app_state.all_input_to_widget
            && let Some(handler) = self.handler_map.get(&(kind, modifiers, code))
        {
            handler(self);
        } else {
            self.main_widget
                .widget_controls(modifiers, code, &mut self.app_state);
        };
        anyhow::Ok(())
    }

    fn exit_app(app: &mut App) {
        app.exit = true;
    }

    fn on_space(app: &mut App) {
        if app.app_state.last_error.is_some() {
            app.app_state.last_error = None;
            return;
        }

        if app.app_state.player.has_next() || !app.app_state.player.stopped() {
            if let Err(e) = app.app_state.player.start_stop() {
                app.app_state.last_error = Some((Clmp3Error::InputError, vec![format!("{e:#}")]))
            }
        } else {
            app.app_state.set_selected_widget(Clmp3Widget::Load);
        }
    }

    fn check_size(&mut self, terminal: &DefaultTerminal) -> anyhow::Result<()> {
        let size = terminal.size()?;
        if size.width < MIN_TERMINAL_WIDTH || size.height < MIN_TERMINAL_HEIGHT {
            self.app_state.last_error = Some((
                Clmp3Error::TerminalSize,
                Vec::from([
                    String::from("Terminal too small"),
                    format!("Minimal width: {MIN_TERMINAL_WIDTH} columns"),
                    format!("Minimal height: {MIN_TERMINAL_HEIGHT} lines"),
                ]),
            ));
        } else if let Some((error, _)) = self.app_state.last_error
            && error == Clmp3Error::TerminalSize
        {
            self.app_state.last_error = None;
        };
        Ok(())
    }

    fn load_current_pic(&mut self) -> Result<(), anyhow::Error> {
        if let Some(metadata) = self.app_state.current_metadata() {
            if let Some(displayed_id) = &self.main_widget.display_widget.current_source_id
                && displayed_id == &metadata.source.id()
            {
                return Ok(());
            }

            self.app_state
                .metadata_loader
                .load_pic(metadata.source.clone());
        }
        self.main_widget.display_widget.update_state(None, None);
        Ok(())
    }

    fn on_cover_loaded(
        &mut self,
        source: Arc<SourceType>,
        dynamic_image: Option<DynamicImage>,
    ) -> Result<(), anyhow::Error> {
        if let Some(current_id) = self.app_state.player.queue.peek().map(|s| s.id())
            && current_id != source.id()
        {
            return Ok(()); // cover isn't current anymore
        }

        self.main_widget
            .display_widget
            .update_state(dynamic_image, Some(source.id()));

        Ok(())
    }
}
