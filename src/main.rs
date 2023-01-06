pub mod caching;
pub mod parsing;
pub mod stateful;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use parsing::{parse_all, parse_series, ScpObject};
use stateful::StatefulList;
use std::{
    env,
    error::Error,
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc::channel, Mutex};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

#[derive(PartialEq, Clone)]
enum WindowSelect {
    Explorer,
    Objects,
}

#[derive(PartialEq, Eq, Clone)]
enum Mode {
    Default,
    Search,
}

#[derive(Clone)]
struct AppStates {
    window: WindowSelect,
    search: String,
    mode: Mode,
    is_load: bool,
    objects: Option<Vec<ScpObject>>,
    objects_items: StatefulList<ScpObject>,
}

#[derive(Clone)]
struct ObjectsLoading {
    objects: Option<Vec<ScpObject>>,
    objects_items: StatefulList<ScpObject>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app: AppStates = AppStates {
        window: WindowSelect::Objects,
        search: String::new(),
        mode: Mode::Default,
        is_load: true,
        objects: None,
        objects_items: StatefulList::new(),
    };

    let objects_loader = Arc::new(Mutex::new(ObjectsLoading {
        objects: None,
        objects_items: StatefulList::new(),
    }));
    let objects_loader2 = Arc::clone(&objects_loader);

    tokio::spawn(async move {
        let mut lock = objects_loader.lock().await;

        if lock.objects.is_none() {
            lock.objects = Some(parse_all().await);
        }

        lock.objects_items = StatefulList::with_items(lock.objects.clone().unwrap());
    });

    // Collect all arguments
    let args: Vec<String> = env::args().collect();

    // If arguments have string debug
    if args.contains(&String::from("debug")) {
        return Ok(());
    }

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(50);
    let res = run_app(&mut terminal, &mut app, tick_rate, objects_loader2).await;

    // restore terminal
    disable_raw_mode()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn search(app: &mut AppStates) {
    if app.is_load {
        return;
    }

    let objects: Vec<ScpObject>;
    if app.search.len() > 0 {
        objects = app
            .objects
            .clone()
            .unwrap()
            .iter()
            .map(|x| x.clone())
            .filter(|o| {
                o.get_document_name()
                    .to_ascii_lowercase()
                    .contains(&app.search.to_ascii_lowercase())
                    || o.get_name()
                        .to_ascii_lowercase()
                        .contains(&app.search.to_ascii_lowercase())
            })
            .collect::<Vec<ScpObject>>();
    } else {
        objects = app.objects.clone().unwrap();
    }

    app.objects_items = StatefulList::with_items(objects);
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppStates,
    tick_rate: Duration,
    objects: Arc<Mutex<ObjectsLoading>>,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let (tx, mut rx) = channel(100);

    tokio::spawn(async move {
        let lock = objects.lock().await;
        loop {
            if lock.objects.is_some() {
                match tx.send(lock.clone()).await {
                    Ok(_) => {}
                    Err(_) => panic!("Fuck!!"),
                }
                break;
            }
        }
    });

    loop {
        terminal.draw(|f| ui(f, app))?;

        match rx.recv().await {
            Some(c) => {
                app.is_load = false;
                app.objects = c.objects.clone();
                app.objects_items = c.objects_items.clone();
            }
            None => {
                rx.close();
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    Mode::Default => match key.code {
                        KeyCode::Esc => {
                            return Ok(());
                        }

                        KeyCode::Right | KeyCode::Left => {
                            if app.mode == Mode::Default {
                                if WindowSelect::eq(&app.window, &WindowSelect::Explorer) {
                                    app.window = WindowSelect::Objects;
                                } else {
                                    app.window = WindowSelect::Explorer;
                                }
                            }
                        }

                        KeyCode::Up => {
                            if !app.is_load {
                                app.objects_items.previous()
                            }
                        }

                        KeyCode::Down => {
                            if !app.is_load {
                                app.objects_items.next()
                            }
                        }

                        KeyCode::Char(c) => {
                            app.mode = Mode::Search;
                            app.objects_items.unselect();
                            app.search.push(c);
                        }

                        KeyCode::Backspace => {
                            app.mode = Mode::Search;
                            app.search.pop();
                        }

                        _ => {}
                    },

                    Mode::Search => match key.code {
                        KeyCode::Esc => {
                            app.mode = Mode::Default;
                        }

                        KeyCode::Char(c) => {
                            app.search.push(c);
                            search(app)
                        }

                        KeyCode::Backspace => {
                            app.search.pop();
                            search(app);
                        }

                        KeyCode::Enter => {
                            app.mode = Mode::Default;
                            app.window = WindowSelect::Objects;
                            search(app);
                            if !app.is_load {
                                app.objects_items.next();
                            }
                        }

                        KeyCode::Right | KeyCode::Left => {
                            app.mode = Mode::Default;
                            if WindowSelect::eq(&app.window, &WindowSelect::Explorer) {
                                app.window = WindowSelect::Objects;
                            } else {
                                app.window = WindowSelect::Explorer;
                            }
                        }

                        KeyCode::Down => {
                            app.mode = Mode::Default;
                            app.window = WindowSelect::Objects;
                            app.objects_items.next();
                        }

                        _ => {}
                    },
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut AppStates) {
    let size = f.size();

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
        .split(size);

    let mut chunks = Layout::default().direction(Direction::Horizontal);

    if app.window == WindowSelect::Objects {
        chunks =
            chunks.constraints([Constraint::Percentage(60), Constraint::Percentage(50)].as_ref());
    } else {
        chunks =
            chunks.constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref());
    }

    let chunks = chunks.split(vertical_chunks[0]);

    let chunk_left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(12), Constraint::Percentage(100)])
        .split(chunks[0]);

    let mut block_with_scp = Block::default().borders(Borders::ALL).title("SCP Объекты");
    let mut block_explorer = Block::default().borders(Borders::ALL).title("Обзор");

    let block_info = Block::default().borders(Borders::ALL);

    let text = vec![Spans::from(vec![
        Span::raw("  "),
        Span::styled("Esc", Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled("Выйти", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("<- ->", Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(
            "Выбрать окно",
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ])];

    let info = Paragraph::new(text)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .block(block_info);

    if app.mode == Mode::Default {
        if WindowSelect::eq(&app.window, &WindowSelect::Explorer) {
            block_explorer = block_explorer.border_style(Style::default().bg(Color::Blue));
        } else {
            block_with_scp = block_with_scp.border_style(Style::default().bg(Color::Blue));
        }
    }

    let mut search_block = Block::default()
        .title("Поиск")
        .border_type(tui::widgets::BorderType::Rounded)
        .borders(Borders::ALL);

    if app.mode == Mode::Search {
        search_block = search_block.border_style(Style::default().bg(Color::Blue));
    }

    let search_widget = Paragraph::new(Span::styled(
        &app.search,
        Style::default().fg(Color::LightGreen),
    ))
    .block(search_block);

    let objects: Vec<ListItem> = app
        .objects_items
        .items
        .iter()
        .map(|o| {
            ListItem::new(format!(
                "[{}] {} - {}",
                o.get_class(),
                o.get_document_name(),
                o.get_name()
            ))
            .style(Style::default().fg(Color::White))
        })
        .collect();

    let scp_list = List::new(objects)
        .block(block_with_scp)
        .highlight_style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("☛");

    /* Search Pane */
    f.render_widget(search_widget, chunk_left[0]);
    // Render block with the SCP objects
    if !app.is_load {
        f.render_stateful_widget(scp_list, chunk_left[1], &mut app.objects_items.state);
    } else {
        let mut block = Block::default()
            .border_style(Style::default())
            .border_type(tui::widgets::BorderType::Rounded)
            .borders(Borders::ALL)
            .title("SCP Объекты (Загружаются)");
        if app.window == WindowSelect::Objects && app.mode == Mode::Default {
            block = block.border_style(Style::default().bg(Color::Blue))
        }

        f.render_widget(block, chunk_left[1]);
    }
    // Render block for explore objects
    f.render_widget(block_explorer, chunks[1]);
    // Render block for see tips for using app
    f.render_widget(info, vertical_chunks[1]);
}
