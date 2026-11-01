use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::widget::{self, text, Column, Row};
use cosmic::{Application, Element};
use serde::Deserialize;

const APP_ID: &str = "io.github.claude_usage";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const POLL_SECS: u64 = 120;

#[derive(Debug, Clone, Deserialize)]
struct Window {
    utilization: f64,
    resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
struct Usage {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
    seven_day_opus: Option<Window>,
    seven_day_sonnet: Option<Window>,
}

#[derive(Deserialize)]
struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    oauth: OAuth,
}

#[derive(Deserialize)]
struct OAuth {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
}

async fn fetch_usage() -> Result<Usage, String> {
    let path = dirs::home_dir()
        .ok_or("nincs HOME")?
        .join(".claude/.credentials.json");
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("credentials: {e}"))?;
    let creds: Credentials = serde_json::from_str(&raw).map_err(|e| format!("credentials: {e}"))?;
    if creds.oauth.expires_at < Utc::now().timestamp_millis() {
        return Err("a token lejárt – indítsd el a `claude`-ot a frissítéshez".into());
    }

    let resp = reqwest::Client::new()
        .get(USAGE_URL)
        .bearer_auth(&creds.oauth.access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "cosmic-applet-claude-usage")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Usage>().await.map_err(|e| e.to_string())
}

#[derive(Default)]
struct App {
    core: Core,
    popup: Option<Id>,
    usage: Option<Usage>,
    error: Option<String>,
    updated: Option<DateTime<Local>>,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(Id),
    Refresh,
    Fetched(Result<Usage, String>),
}

fn fetch() -> Task<Message> {
    Task::perform(fetch_usage(), |r| cosmic::Action::App(Message::Fetched(r)))
}

fn pct(w: &Option<Window>) -> String {
    w.as_ref()
        .map(|w| format!("{:.0}%", w.utilization))
        .unwrap_or_else(|| "–".into())
}

fn reset_label(w: &Window) -> String {
    let Some(at) = w.resets_at else {
        return String::new();
    };
    let left = at - Utc::now();
    let mins = left.num_minutes().max(0);
    let local = at.with_timezone(&Local);
    if mins < 24 * 60 {
        format!("reset {}h {}m múlva ({})", mins / 60, mins % 60, local.format("%H:%M"))
    } else {
        format!("reset {}n {}h múlva ({})", mins / 1440, (mins % 1440) / 60, local.format("%m.%d. %H:%M"))
    }
}

fn section<'a>(title: &'a str, w: &'a Option<Window>) -> Option<Element<'a, Message>> {
    let w = w.as_ref()?;
    Some(
        Column::new()
            .spacing(4)
            .push(
                Row::new()
                    .push(text::heading(title))
                    .push(widget::space().width(Length::Fill))
                    .push(text::heading(format!("{:.0}%", w.utilization))),
            )
            .push(widget::determinate_linear((w.utilization / 100.0).clamp(0.0, 1.0) as f32).width(Length::Fill))
            .push(text::caption(reset_label(w)))
            .into(),
    )
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        let app = App { core, ..Default::default() };
        (app, fetch())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Message> {
        cosmic::iced::time::every(Duration::from_secs(POLL_SECS)).map(|_| Message::Refresh)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Refresh => return fetch(),
            Message::Fetched(Ok(u)) => {
                self.usage = Some(u);
                self.error = None;
                self.updated = Some(Local::now());
            }
            Message::Fetched(Err(e)) => self.error = Some(e),
            Message::TogglePopup => {
                if let Some(id) = self.popup.take() {
                    return cosmic::surface::surface_task(cosmic::surface::action::destroy_popup(id));
                }
                // Open through libcosmic's surface action (as the stock applets do) so
                // the popup gets the theme's frosted blur and rounded corners.
                let popup = cosmic::surface::surface_task(cosmic::surface::action::app_popup(
                    |_: &Self| Default::default(),
                    |app: &mut Self| {
                        let id = Id::unique();
                        app.popup = Some(id);
                        let mut settings = app.core.applet.get_popup_settings(
                            app.core.main_window_id().unwrap(),
                            id,
                            None,
                            None,
                            None,
                        );
                        settings.positioner.size_limits = Limits::NONE
                            .min_width(280.0)
                            .max_width(360.0)
                            .min_height(100.0)
                            .max_height(600.0);
                        settings
                    },
                    None,
                ));
                return Task::batch([popup, fetch()]);
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let label = match (&self.usage, &self.error) {
            (Some(u), _) => format!("5h {}  ·  7d {}", pct(&u.five_hour), pct(&u.seven_day)),
            (None, Some(_)) => "Claude ⚠".into(),
            (None, None) => "Claude …".into(),
        };
        let content = Row::new()
            .align_y(Alignment::Center)
            .spacing(6)
            .push(self.core.applet.text(label));
        widget::button::custom(self.core.applet.autosize_window(content))
            .class(cosmic::theme::Button::AppletIcon)
            .padding([0, self.core.applet.suggested_padding(true).0])
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        let mut col = Column::new().spacing(14).padding(16).width(Length::Fill);
        col = col.push(text::title4("Claude felhasználás"));
        if let Some(u) = &self.usage {
            for (title, w) in [
                ("Session (5 óra)", &u.five_hour),
                ("Heti (7 nap)", &u.seven_day),
                ("Heti – Opus", &u.seven_day_opus),
                ("Heti – Sonnet", &u.seven_day_sonnet),
            ] {
                if let Some(s) = section(title, w) {
                    col = col.push(s);
                }
            }
        }
        if let Some(e) = &self.error {
            col = col.push(text::body(format!("⚠ {e}")));
        }
        let footer = self
            .updated
            .map(|t| format!("Frissítve: {}", t.format("%H:%M:%S")))
            .unwrap_or_default();
        col = col.push(
            Row::new()
                .align_y(Alignment::Center)
                .push(text::caption(footer))
                .push(widget::space().width(Length::Fill))
                .push(widget::button::standard("Frissítés").on_press(Message::Refresh)),
        );
        self.core.applet.popup_container(col).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<App>(())
}
