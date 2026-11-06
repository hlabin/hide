mod backend;

use backend::{Project, ProjectAction, Service, ServiceAction};
use gpui::{
    App, Bounds, Context, IntoElement, QuitMode, Render, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, rgb, size,
};

#[derive(Clone, Copy)]
struct Palette {
    primary: u32,
    sidebar: u32,
    secondary: u32,
    hover: u32,
    active: u32,
    text: u32,
    muted: u32,
    border: u32,
    terminal: u32,
}

impl Palette {
    fn for_dark(dark: bool) -> Self {
        if dark {
            Self {
                primary: 0x1a1a1a, sidebar: 0x1e1e1e, secondary: 0x242424,
                hover: 0x2a2a2a, active: 0x333333, text: 0xe5e5e5,
                muted: 0x919191, border: 0x2e2e2e, terminal: 0x1a1a1a,
            }
        } else {
            Self {
                primary: 0xffffff, sidebar: 0xfafafa, secondary: 0xf5f5f5,
                hover: 0xf0f0f0, active: 0xe8e8e8, text: 0x1a1a1a,
                muted: 0x6b6b6b, border: 0xe5e5e5, terminal: 0xfafafa,
            }
        }
    }
}

struct HideApp {
    projects: Vec<Project>,
    selected: Option<String>,
    services: Vec<Service>,
    service_error: Option<String>,
    error: Option<String>,
    dark: bool,
}

impl HideApp {
    fn new() -> Self {
        let mut this = Self {
            projects: vec![], selected: None, services: vec![], service_error: None, error: None, dark: true,
        };
        this.refresh();
        this
    }

    fn refresh(&mut self) {
        match backend::list_projects() {
            Ok(projects) => {
                if !self.selected.as_ref().is_some_and(|name| projects.iter().any(|p| &p.name == name)) {
                    self.selected = projects.first().map(|p| p.name.clone());
                }
                self.projects = projects;
                self.error = None;
                self.refresh_services();
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn refresh_services(&mut self) {
        let Some(name) = self.selected.as_deref() else {
            self.services.clear();
            self.service_error = None;
            return;
        };
        match backend::list_services(name) {
            Ok(services) => {
                self.services = services;
                self.service_error = None;
            }
            Err(error) => {
                self.services.clear();
                self.service_error = Some(error);
            }
        }
    }
}

impl Render for HideApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::for_dark(self.dark);
        let selected = self.projects.iter().find(|p| Some(&p.name) == self.selected.as_ref());
        let project_title = selected.map(|p| {
            if p.label.is_empty() { p.name.clone() } else { p.label.clone() }
        }).unwrap_or_else(|| "No project selected".into());
        let project_root = selected.map(|p| p.root.clone()).unwrap_or_default();
        let selected_name = self.selected.clone();
        let project_running = selected.is_some_and(|p| p.running);
        let project_error = selected.and_then(|p| p.error.clone());
        let project_agents = selected.and_then(|p| p.agent_summary());
        let project_kind = selected.map(|p| match (p.is_remote, p.worktree) {
            (true, true) => "Remote worktree",
            (true, false) => "Remote project",
            (false, true) => "Local worktree",
            (false, false) => "Local project",
        }).unwrap_or("No project");
        let service_status = selected.map(|p| format!("{}/{} services running", p.services_running, p.services_total))
            .unwrap_or_else(|| "Add a project in the existing lpm application".into());

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(c.primary))
            .text_color(rgb(c.text))
            .text_sm()
            .child(
                div()
                    .h(px(42.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(rgb(c.border))
                    .child("hide")
                    .child(
                        div().flex().gap_2()
                            .child(
                                div().id("refresh").cursor_pointer().px_2().py_1()
                                    .rounded_md().bg(rgb(c.hover))
                                    .child("Refresh")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh();
                                        cx.notify();
                                    }))
                            )
                            .child(
                                div().id("theme").cursor_pointer().px_2().py_1()
                                    .rounded_md().bg(rgb(c.hover))
                                    .child(if self.dark { "Light" } else { "Dark" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dark = !this.dark;
                                        cx.notify();
                                    }))
                            )
                    )
            )
            .child(
                div().flex().flex_1().min_h_0()
                    .child(
                        div().w(px(250.)).h_full().flex().flex_col()
                            .bg(rgb(c.sidebar))
                            .border_r_1().border_color(rgb(c.border))
                            .child(
                                div().px_4().py_3().text_color(rgb(c.muted))
                                    .child("PROJECTS")
                            )
                            .child(
                                div().flex_1().overflow_y_scroll()
                                    .children(self.projects.iter().map(|p| {
                                        let name = p.name.clone();
                                        let picked = self.selected.as_deref() == Some(name.as_str());
                                        let label = if p.label.is_empty() { p.name.clone() } else { p.label.clone() };
                                        div()
                                            .id(("project", name.clone()))
                                            .mx_2().mb_1().px_3().py_2().rounded_md()
                                            .cursor_pointer()
                                            .bg(rgb(if picked { c.active } else { c.sidebar }))
                                            .hover(move |s| s.bg(rgb(c.hover)))
                                            .flex().items_center().gap_2()
                                            .child(
                                                div().text_color(rgb(if p.running { 0x4ade80 } else { c.muted }))
                                                    .child(if p.running { "●" } else { "○" })
                                            )
                                            .child(label)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.selected = Some(name.clone());
                                                this.refresh_services();
                                                cx.notify();
                                            }))
                                    }))
                            )
                            .child(
                                div().px_4().py_3().border_t_1().border_color(rgb(c.border))
                                    .text_color(rgb(c.muted))
                                    .child(format!("{} projects", self.projects.len()))
                            )
                    )
                    .child(
                        div().flex_1().flex().flex_col().min_w_0()
                            .child(
                                div().px_6().py_4().border_b_1().border_color(rgb(c.border))
                                    .child(project_title)
                            )
                            .child(
                                div().px_6().py_3().text_color(rgb(c.muted))
                                    .child(project_root)
                            )
                            .child(
                                div().px_6().py_2().child(service_status)
                            )
                            .child(
                                div().px_6().py_1().text_color(rgb(c.muted)).child(project_kind)
                            )
                            .children(project_agents.into_iter().map(|agents| {
                                div().px_6().py_1().text_color(rgb(c.muted))
                                    .child(format!("Agents: {agents}"))
                            }))
                            .children(project_error.into_iter().map(|error| {
                                div().px_6().py_1().text_color(rgb(0xf87171))
                                    .child(format!("Project config: {error}"))
                            }))
                            .child(
                                div().px_6().py_2().flex().gap_2()
                                    .child(
                                        div().id("project-control")
                                            .cursor_pointer().px_3().py_2()
                                            .rounded_md().bg(rgb(c.hover))
                                            .child(if project_running { "Stop project" } else { "Start project" })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(name) = selected_name.as_deref() {
                                                    let action = if project_running { ProjectAction::Stop } else { ProjectAction::Start };
                                                    match backend::change_project_state(name, action) {
                                                        Ok(()) => this.refresh(),
                                                        Err(error) => this.error = Some(error),
                                                    }
                                                    cx.notify();
                                                }
                                            }))
                                    )
                            )
                            .child(
                                div().px_6().py_2().text_color(rgb(c.muted))
                                    .child("SERVICES")
                            )
                            .children(self.services.iter().map(|service| {
                                let service_name = service.name.clone();
                                let project_name = self.selected.clone().unwrap_or_default();
                                let running = service.running;
                                let port = if service.port > 0 {
                                    format!(" :{}", service.port)
                                } else {
                                    String::new()
                                };
                                div().mx_6().mb_1().px_3().py_2().rounded_md()
                                    .bg(rgb(c.secondary)).flex().items_center().justify_between()
                                    .child(
                                        div().flex().gap_2().items_center()
                                            .child(
                                                div().text_color(rgb(if running { 0x4ade80 } else { c.muted }))
                                                    .child(if running { "●" } else { "○" })
                                            )
                                            .child(format!("{}{}", service.name, port))
                                    )
                                    .child(
                                        div().id(("service-control", service.name.clone()))
                                            .cursor_pointer().rounded_md().bg(rgb(c.hover))
                                            .px_2().py_1()
                                            .child(if running { "Stop" } else { "Start" })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let action = if running { ServiceAction::Stop } else { ServiceAction::Start };
                                                match backend::change_service_state(&project_name, &service_name, action) {
                                                    Ok(()) => this.refresh(),
                                                    Err(error) => this.service_error = Some(error),
                                                }
                                                cx.notify();
                                            }))
                                    )
                            }))
                            .children(self.service_error.iter().map(|error| {
                                div().px_6().py_2().text_color(rgb(0xf87171))
                                    .child(format!("Services: {error}"))
                            }))
                            .child(
                                div().flex_1().m_6().rounded_md()
                                    .border_1().border_color(rgb(c.border))
                                    .bg(rgb(c.terminal))
                                    .flex().flex_col()
                                    .child(
                                        div().px_3().py_2().bg(rgb(c.secondary))
                                            .border_b_1().border_color(rgb(c.border))
                                            .child("Terminal")
                                    )
                                    .child(
                                        div().px_4().py_4().text_color(rgb(c.muted))
                                            .child("Native terminal/PTY rendering is not yet migrated.")
                                    )
                            )
                            .children(self.error.iter().map(|e| {
                                div().px_6().py_2().text_color(rgb(0xf87171)).child(e.clone())
                            }))
                    )
            )
    }
}

fn main() {
    gpui_platform::application()
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1200.), px(780.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| HideApp::new()),
            ).expect("open Hide GPUI window");
            cx.activate(true);
        });
}
