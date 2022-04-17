use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};

use std::{
    error::Error,
    io,
    time::{Duration, Instant}
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, canvas::{Canvas, Line, Rectangle}, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal
};

mod moostar;

enum InputEditionMode {
    Normal,
    Editing
}

#[derive(Copy, Clone)]
enum Frequency {
    HalfHz,
    OneHz,
    TwoHz,
    FiveHz,
    TenHz,
    TwentyHz,
    FiftyHz,
    HundredHz,
    TwoHundredHz,
    FiveHundredHz,
    ThousandHz
}

struct App {
    /// Runner
    runner: moostar::Runner,
    /// Keep a separate, original version of the code here
    code: String,
    /// Input Edition Mode
    edition_mode: InputEditionMode,
    /// The Frequency we are set at
    frequency: Frequency,
    /// Running
    running: bool,
    /// Debug
    funny_number: u16
}

impl App {
    fn new(path: &str) -> Result<App, Box<dyn Error>> {
        let vecbytes: Vec<u8> = std::fs::read(path)?;
        let decoded: String = String::from_utf8(vecbytes)?;
        Ok(App {
            runner: moostar::Runner::new(&decoded)?,
            code: decoded + " ", // That space serves for "halt"
            edition_mode: InputEditionMode::Normal,
            frequency: Frequency::OneHz,
            running: false,
            funny_number: 0
        })
    }

    fn step(&mut self) -> () {
        self.runner.step();
    }

    fn get_input(&self) -> &str {
        &self.runner.get_input()
    }

    fn get_code(&self) -> &str {
        &self.code
    }

    fn get_jumps(&self, max_of: Option<usize>) -> Text {
        let style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);
        Text::from(
            self.runner
                .jump_list(max_of)
                .iter()
                .map(|pos| Spans::from(vec![
                    Span::raw("Back to #"),
                    Span::styled(pos.to_string(), style)
                ]))
                .collect::<Vec<Spans>>()
        )
    }

    fn get_coloured_code(&self) -> Text {
        // Find out where to split
        let mut colour_span: (usize, usize) = self.runner.get_instruction_span();
        let highlight_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        // Split the code into texts
        let mut spans: Vec<Spans> = Vec::new();
        let mut split_reached: bool = false;
        for line in self.code.lines() {
            // If the remainder of the line is more than the first split, ret
            if colour_span.0 > line.len() {
                colour_span.0 -= line.len() + 1;
                spans.push(Spans::from(vec![Span::raw(line)]));
            } else if split_reached || self.runner.is_halted() {
                spans.push(Spans::from(vec![Span::raw(line)]));
            } else {
                //Split into parts
                let (one, bet) = line.split_at(colour_span.0);
                let (two, three) = bet.split_at(colour_span.1);
                spans.push(Spans::from(vec![Span::raw(one), Span::styled(two, highlight_style), Span::raw(three)]));
                split_reached = true;
            }
        }
        Text::from(spans)
    }

    fn format_ribbon<'a>(&self) -> Span<'a> {
        // So
        // What is the span we have in front of us?
        Span::styled(format!("|{}", (0..100).map(|x| format!(" {:03} ", x)).collect::<Vec<String>>().join("|")), Style::default())
    }

    fn get_freq_list_state(&self) -> ListState {
        let mut state: ListState = ListState::default();
        state.select(Some(self.frequency as usize));
        state
    }

    fn decrease_frequency(&mut self) {
        self.frequency = match self.frequency {
            Frequency::HalfHz => Frequency::HalfHz,
            Frequency::OneHz => Frequency::HalfHz,
            Frequency::TwoHz => Frequency::OneHz,
            Frequency::FiveHz => Frequency::TwoHz,
            Frequency::TenHz => Frequency::FiveHz,
            Frequency::TwentyHz => Frequency::TenHz,
            Frequency::FiftyHz => Frequency::TwentyHz,
            Frequency::HundredHz => Frequency::FiftyHz,
            Frequency::TwoHundredHz => Frequency::HundredHz,
            Frequency::FiveHundredHz => Frequency::TwoHundredHz,
            Frequency::ThousandHz => Frequency::FiveHundredHz
        }
    }

    fn increase_frequency(&mut self) {
        self.frequency = match self.frequency {
            Frequency::HalfHz => Frequency::OneHz,
            Frequency::OneHz => Frequency::TwoHz,
            Frequency::TwoHz => Frequency::FiveHz,
            Frequency::FiveHz => Frequency::TenHz,
            Frequency::TenHz => Frequency::TwentyHz,
            Frequency::TwentyHz => Frequency::FiftyHz,
            Frequency::FiftyHz => Frequency::HundredHz,
            Frequency::HundredHz => Frequency::TwoHundredHz,
            Frequency::TwoHundredHz => Frequency::FiveHundredHz,
            Frequency::FiveHundredHz => Frequency::ThousandHz,
            Frequency::ThousandHz => Frequency::ThousandHz
        }
    }

    fn list_frequencies(&self) -> Vec<ListItem> {
        vec![
            ListItem::new("1/2 Hz"),
            ListItem::new("1 Hz"), ListItem::new("2 Hz"), ListItem::new("5 Hz"),
            ListItem::new("10 Hz"), ListItem::new("20 Hz"), ListItem::new("50 Hz"),
            ListItem::new("100 Hz"), ListItem::new("200 Hz"), ListItem::new("500 Hz"),
            ListItem::new("1000 Hz")
        ]
    }

    fn get_delay(&self) -> Duration {
        Duration::from_millis(
            match self.frequency {
                Frequency::HalfHz => 2000,
                Frequency::OneHz => 1000,
                Frequency::TwoHz => 500,
                Frequency::FiveHz => 200,
                Frequency::TenHz => 100,
                Frequency::TwentyHz => 50,
                Frequency::FiftyHz => 20,
                Frequency::HundredHz => 10,
                Frequency::TwoHundredHz => 5,
                Frequency::FiveHundredHz => 2,
                Frequency::ThousandHz => 1
            }
        )
    }
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn disable_terminal<B: Backend + std::io::Write>(mut terminal: Terminal<B>) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Fetch argument
    let mut args = std::env::args();
    if args.len() < 2 {
        eprintln!("Provide a file path please");
        return Ok(());
    }
    // Set it up
    let mut terminal = init_terminal()?;
    let app = App::new(&args.nth(1).unwrap())?;
    let res = run_app(&mut terminal, app);

    // restore it
    disable_terminal(terminal)?;

    if let Err(err) = res {
        eprintln!("Shoot!\n{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Find the tick rate from app
        let timeout = app.get_delay()
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));
        // Use all of that remaining time to try and fetch a key event
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => app.decrease_frequency(),
                    KeyCode::Down => app.increase_frequency(),
                    KeyCode::Char(' ') => app.running = !app.running,
                    KeyCode::Char('s') => { app.running = false; app.step(); }
                    _ => {}
                }
            }
        }
        // If we haven't reached the tick rate, don't tick, otherwise tick
        let delay = app.get_delay().as_millis();
        let elapsed = last_tick.elapsed().as_millis();
        if elapsed >= delay {
            // app.tick();
            // Compute how many ticks must be done at once
            let num_of_ticks: u128 = elapsed.div_euclid(delay);
            let rem: u128 = elapsed % delay;
            last_tick = Instant::now() - Duration::from_millis(rem as u64);
            // Do the ticks
            if app.running {
                (0..num_of_ticks).for_each(|_| app.step());
                app.funny_number = app.funny_number.wrapping_add(num_of_ticks as u16);    
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    // Wrapping block for a group
    // Just draw the block and the group on the same area and build the group
    // with at least a margin of 1
    let size = f.size();

    // Suddounding block
    let block = Block::default()
        .borders(Borders::TOP)
        .title("Nose Burn 👃🔥")
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Thick);
    f.render_widget(block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Length(3), Constraint::Min(10), Constraint::Length(3)].as_ref())
        .split(f.size());

    let canvas_block = Canvas::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Ribbons", Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC)))
            .title_alignment(Alignment::Right))
        .paint(|ctx| {
            let spanned: Span = app.format_ribbon();
            ctx.print(0.0, 100.0, spanned);
        })
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0]);
    f.render_widget(canvas_block, chunks[0]);

    let input_block = Paragraph::new(app.get_input())
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::reset())
            .title(Span::styled("Input", Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC)))
            .title_alignment(Alignment::Right)
            .border_type(BorderType::Plain))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left);
    f.render_widget(input_block, chunks[1]);

    let detail_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(50), Constraint::Percentage(20)].as_ref())
        .split(chunks[2]);

    let jump_block = Paragraph::new(app.get_jumps(Some((detail_chunks[0].height-2).into())))
        .block(Block::default()
            .borders(Borders::ALL)
            .title("-::[Jumps]::-")
            .title_alignment(Alignment::Center));
    f.render_widget(jump_block, detail_chunks[0]);

    let code_block = Paragraph::new(app.get_coloured_code())
        .block(Block::default()
            .borders(Borders::ALL)
            .title("-::[Code]::-")
            .title_alignment(Alignment::Center))
        .wrap(Wrap { trim: false });
    f.render_widget(code_block, detail_chunks[1]);

    let freq_list = List::new(app.list_frequencies())
        .block(Block::default().title(":[Frequency]:").title_alignment(Alignment::Center).borders(Borders::ALL))
        .style(Style::default())
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">");
    let mut state: ListState = ListState::default();
    state.select(Some(0));
    f.render_stateful_widget(freq_list, detail_chunks[2], &mut app.get_freq_list_state());

    let help_block = Paragraph::new(format!("Q: Quit    S: Step    Space: {}\nUp: Lower Frequency    Down: Increase Frequency", if app.running { "Pause"  } else { "Start" }))
        .block(Block::default()
            .borders(Borders::TOP)
            .title(Span::styled("Keys", Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC)))
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Plain))
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Center);
    f.render_widget(help_block, chunks[3]);
}
