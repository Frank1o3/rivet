use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, InputMode};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Search bar
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Footer / Status bar
        ])
        .split(f.area());

    render_header(f, chunks[0]);
    render_search(f, app, chunks[1]);
    render_main(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);

    if app.show_modal {
        render_plan_modal(f, app, f.area());
    }
}

fn render_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " 🔩 RIVET ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "Package Manager Explorer",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    f.render_widget(title, area);
}

fn render_search(f: &mut Frame, app: &App, area: Rect) {
    let (border_color, prompt_prefix) = match app.input_mode {
        InputMode::Search => (Color::Yellow, "Search [ACTIVE]: "),
        InputMode::Normal => (Color::DarkGray, "Search (press [/] to type): "),
    };

    let text = Line::from(vec![
        Span::styled(
            prompt_prefix,
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&app.search_query),
    ]);

    let search_bar = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(search_bar, area);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left: Package List
    let items: Vec<ListItem> = app
        .filtered_packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            let is_selected = i == app.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { " ▶ " } else { "   " };
            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(pkg.name.as_str(), style),
                Span::raw(" "),
                Span::styled(
                    format!("v{}", pkg.version),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list_title = format!(" Packages ({}) ", app.filtered_packages.len());
    let list_widget = List::new(items).block(
        Block::default()
            .title(list_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list_widget, chunks[0]);

    // Right: Package Inspector
    render_inspector(f, app, chunks[1]);
}

fn render_inspector(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Package Inspector ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let content = if let Some(pkg) = app.selected_package() {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled(
                "Name:         ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                pkg.name.as_str(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Version:      ", Style::default().fg(Color::Cyan)),
            Span::raw(pkg.version.to_string()),
        ]));

        if let Some(desc) = &pkg.description {
            lines.push(Line::from(vec![
                Span::styled("Description:  ", Style::default().fg(Color::Cyan)),
                Span::raw(desc),
            ]));
        }

        if let Some(lic) = &pkg.license {
            lines.push(Line::from(vec![
                Span::styled("License:      ", Style::default().fg(Color::Cyan)),
                Span::raw(lic),
            ]));
        }

        if let Some(publ) = &pkg.publisher {
            lines.push(Line::from(vec![
                Span::styled("Publisher:    ", Style::default().fg(Color::Cyan)),
                Span::raw(publ),
            ]));
        }

        if let Some(src) = &pkg.source {
            lines.push(Line::from(vec![
                Span::styled("Source:       ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:?}", src)),
            ]));
        }

        lines.push(Line::raw(""));

        // Dependencies
        lines.push(Line::from(vec![Span::styled(
            "Dependencies:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        if pkg.dependencies.is_empty() {
            lines.push(Line::raw("  (none)"));
        } else {
            for dep in &pkg.dependencies {
                let kind_tag = format!("({:?})", dep.kind);
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(dep.to_string(), Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(kind_tag, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        lines.push(Line::raw(""));

        // Features
        lines.push(Line::from(vec![Span::styled(
            "Features:",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]));
        if pkg.features.is_empty() {
            lines.push(Line::raw("  (none)"));
        } else {
            for (feat, deps) in &pkg.features {
                let is_default = pkg.default_features.contains(feat);
                let tag = if is_default { " [default]" } else { "" };
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(feat.as_str(), Style::default().fg(Color::White)),
                    Span::styled(tag, Style::default().fg(Color::Yellow)),
                ]));
                for d in deps {
                    lines.push(Line::from(vec![Span::styled(
                        format!("      requires: {}", d),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
            }
        }

        Paragraph::new(lines).block(block).wrap(Wrap { trim: true })
    } else {
        Paragraph::new("No package selected.").block(block)
    };

    f.render_widget(content, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.input_mode {
        InputMode::Search => "[SEARCH MODE] Type query | [Enter/Esc] Done | [Backspace] Delete",
        InputMode::Normal => {
            "[q] Quit | [/] Search | [↑/↓] Navigate | [Enter] Resolve Plan | [s] Sync Repos"
        }
    };

    let text = Line::from(vec![
        Span::styled(
            format!(" {} ", app.status_message),
            Style::default().fg(Color::White),
        ),
        Span::raw("  │  "),
        Span::styled(mode_indicator, Style::default().fg(Color::DarkGray)),
    ]);

    let footer = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    f.render_widget(footer, area);
}

fn render_plan_modal(f: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(65, 60, area);
    f.render_widget(Clear, modal_area);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "📦 Resolution Plan (Installation Sequence):",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::raw(""));

    if let Some(plan) = &app.resolution_plan {
        for (i, item) in plan.iter().enumerate() {
            let build_tag = if item.build_dependencies.is_empty() {
                String::new()
            } else {
                format!(
                    " [build-deps: {}]",
                    item.build_dependencies
                        .iter()
                        .map(|d| d.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::Cyan)),
                Span::styled(
                    item.manifest.name.as_str(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("v{}", item.manifest.version),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(build_tag, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![Span::styled(
        "Press [Esc] or [Enter] to close this dialog.",
        Style::default().fg(Color::DarkGray),
    )]));

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" Dependency Plan ")
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(popup, modal_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
