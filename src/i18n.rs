//! Interface text in English and Brazilian Portuguese.

use chrono::{Datelike, Duration, NaiveDate};
use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Lang {
    En,
    Pt,
}

pub type Keys = &'static [(&'static str, &'static str)];

pub struct Text {
    pub local_time: &'static str,
    pub local_tab: &'static str,
    pub city: &'static str,
    pub city_prompt: &'static str,
    pub no_match: &'static str,
    pub alarms: &'static str,
    pub no_alarms: &'static str,
    pub new_alarm: &'static str,
    pub alarm_example: &'static str,
    pub invalid_time: &'static str,
    pub map_no_room: &'static str,
    pub themes: &'static str,
    pub calendar: &'static str,
    pub calendar_address: &'static str,
    pub source: &'static str,
    pub mac_calendar: &'static str,
    pub mac_calendar_about: &'static str,
    pub ical_about: &'static str,
    pub asking_access: &'static str,
    pub access_denied: &'static str,
    pub access_not_asked: &'static str,
    pub access_not_shown: &'static str,
    pub no_events_mac: &'static str,
    pub calendar_how: [&'static str; 3],
    pub next_event: &'static str,
    pub reminder: &'static str,
    pub minutes_before: &'static str,
    pub calendar_unavailable: &'static str,
    pub no_events: &'static str,
    pub updated_at: &'static str,
    pub rain: &'static str,
    pub at: &'static str,
    pub no_rain: &'static str,
    pub weather_unavailable: &'static str,
    pub loading_weather: &'static str,
    pub home_city: &'static str,
    pub no_theme: &'static str,
    pub unsaved: &'static str,
    pub daily: &'static str,
    pub once: &'static str,
    pub off: &'static str,
    pub alarm: &'static str,
    pub next_alarm: &'static str,
    pub snooze: &'static str,
    pub local: &'static str,
    pub help: &'static str,
    pub in_: &'static str,
    pub clock_keys: Keys,
    pub search_keys: Keys,
    pub alarm_keys: Keys,
    pub input_keys: Keys,
    pub ringing_keys: Keys,
    pub theme_keys: Keys,
    pub calendar_keys: Keys,
    pub mac_calendar_keys: Keys,
    pub choose_keys: Keys,
    pub help_keys: Keys,
    weekdays: [&'static str; 7],
    months: [&'static str; 12],
}

const EN: Text = Text {
    local_time: "Local time",
    local_tab: "Local",
    city: "City",
    city_prompt: "Type a city: Tokyo, São Paulo, Portland, Maine…",
    no_match: "No city found",
    alarms: "Alarms",
    no_alarms: "No alarms yet. Press n to add one.",
    new_alarm: "New alarm",
    alarm_example: "07:30, 7h30, 7:30pm or +10m, then an optional label",
    invalid_time: "Invalid time. Try 07:30, 7:30pm or +10m",
    map_no_room: "The map needs a bigger window",
    themes: "Themes",
    calendar: "Calendar",
    calendar_address: "iCal address",
    source: "Source",
    mac_calendar: "Mac Calendar",
    mac_calendar_about: "events of the accounts in the Calendar app, Google included",
    ical_about: "any calendar that offers an .ics address",
    asking_access: "Waiting for macOS: allow access to your calendars",
    access_denied: "macOS denied access to the calendars. o opens the setting to allow your terminal",
    access_not_asked: "Enter asks macOS for access to your calendars",
    access_not_shown: "macOS did not show the request",
    no_events_mac: "Is your Google account in the Calendar app? a opens Internet Accounts",
    calendar_how: [
        "Paste your calendar's secret iCal address:",
        "Google Calendar › Settings › (your calendar) ›",
        "Integrate calendar › Secret address in iCal format",
    ],
    next_event: "Next event",
    reminder: "Reminder",
    minutes_before: "min before",
    calendar_unavailable: "Calendar unavailable",
    no_events: "No events in the next 7 days",
    updated_at: "Updated at",
    rain: "rain",
    at: "at",
    no_rain: "no rain in the next 12 h",
    weather_unavailable: "Weather unavailable",
    loading_weather: "Loading the forecast…",
    home_city: "Your city · weather in the local time tab",
    no_theme: "No theme found",
    unsaved: "Could not save the state",
    daily: "daily",
    once: "once",
    off: "off",
    alarm: "ALARM",
    next_alarm: "Alarm",
    snooze: "snooze",
    local: "local",
    help: "Keys",
    in_: "in",
    clock_keys: &[
        ("c", "city"),
        ("←→", "tabs"),
        ("x", "close"),
        ("m", "map"),
        ("v", "map style"),
        ("t", "theme"),
        ("w", "weather"),
        ("g", "calendar"),
        ("a", "alarms"),
        ("s", "seconds"),
        ("?", "help"),
        ("q", "quit"),
    ],
    search_keys: &[("↑↓", "choose"), ("Enter", "select"), ("Esc", "cancel")],
    alarm_keys: &[
        ("n", "new"),
        ("Space", "on/off"),
        ("r", "daily"),
        ("d", "delete"),
        ("Esc", "close"),
    ],
    input_keys: &[("Enter", "save"), ("Esc", "cancel")],
    ringing_keys: &[("any key", "stop"), ("z", "snooze 5 min")],
    theme_keys: &[("↑↓", "try"), ("Enter", "use"), ("Esc", "cancel")],
    calendar_keys: &[
        ("e", "edit address"),
        ("+ -", "reminder"),
        ("r", "refresh"),
        ("d", "disconnect"),
        ("Esc", "close"),
    ],
    mac_calendar_keys: &[
        ("+ -", "reminder"),
        ("r", "refresh"),
        ("o", "privacy"),
        ("a", "accounts"),
        ("d", "disconnect"),
        ("Esc", "close"),
    ],
    choose_keys: &[("↑↓", "choose"), ("Enter", "connect"), ("Esc", "close")],
    help_keys: &[
        ("c  /", "add a city (it takes the place of local time)"),
        ("←  →", "switch cities (also Tab, 1-9)"),
        ("x", "close the city; the last one brings back local time"),
        ("m", "show or hide the map in this tab"),
        ("a", "alarms, in the zone of this tab"),
        ("v", "map style: ASCII, braille, blocks"),
        ("t", "clock theme: 50 editor themes"),
        ("w  W", "show or hide the weather; W sets your city"),
        ("g", "calendar: Mac Calendar or iCal, reminder 5 min before"),
        ("s", "show or hide seconds"),
        ("q  Ctrl+C", "quit"),
        ("", ""),
        ("", "When an alarm rings the screen blinks:"),
        ("", "any key stops it, z snoozes 5 minutes."),
    ],
    weekdays: [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ],
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
};

const PT: Text = Text {
    local_time: "Hora local",
    local_tab: "Local",
    city: "Cidade",
    city_prompt: "Digite uma cidade: Tóquio, São Paulo, Porto, Portugal…",
    no_match: "Nenhuma cidade encontrada",
    alarms: "Alarmes",
    no_alarms: "Nenhum alarme. Tecle n para criar um.",
    new_alarm: "Novo alarme",
    alarm_example: "07:30, 7h30 ou +10m, e um nome opcional",
    invalid_time: "Horário inválido. Tente 07:30, 7h30 ou +10m",
    map_no_room: "O mapa não cabe nesta janela",
    themes: "Temas",
    calendar: "Agenda",
    calendar_address: "Endereço iCal",
    source: "Fonte",
    mac_calendar: "Calendário do Mac",
    mac_calendar_about: "eventos das contas do app Calendário, Google incluída",
    ical_about: "qualquer agenda que ofereça um endereço .ics",
    asking_access: "Aguardando o macOS: permita o acesso aos seus calendários",
    access_denied: "O macOS negou o acesso aos calendários. o abre o ajuste para liberar seu terminal",
    access_not_asked: "Enter pede ao macOS acesso aos seus calendários",
    access_not_shown: "O macOS não mostrou o pedido",
    no_events_mac: "Sua conta Google está no app Calendário? a abre as Contas de Internet",
    calendar_how: [
        "Cole o endereço secreto iCal da sua agenda:",
        "Google Agenda › Configurações › (sua agenda) ›",
        "Integrar agenda › Endereço secreto no formato iCal",
    ],
    next_event: "Próximo evento",
    reminder: "Lembrete",
    minutes_before: "min antes",
    calendar_unavailable: "Agenda indisponível",
    no_events: "Nenhum evento nos próximos 7 dias",
    updated_at: "Atualizada às",
    rain: "chuva",
    at: "às",
    no_rain: "sem chuva nas próximas 12 h",
    weather_unavailable: "Previsão indisponível",
    loading_weather: "Carregando a previsão…",
    home_city: "Sua cidade · previsão na aba da hora local",
    no_theme: "Nenhum tema encontrado",
    unsaved: "Não foi possível salvar o estado",
    daily: "diário",
    once: "uma vez",
    off: "desligado",
    alarm: "ALARME",
    next_alarm: "Alarme",
    snooze: "soneca",
    local: "local",
    help: "Teclas",
    in_: "em",
    clock_keys: &[
        ("c", "cidade"),
        ("←→", "abas"),
        ("x", "fechar"),
        ("m", "mapa"),
        ("v", "estilo do mapa"),
        ("t", "tema"),
        ("w", "tempo"),
        ("g", "agenda"),
        ("a", "alarmes"),
        ("s", "segundos"),
        ("?", "ajuda"),
        ("q", "sair"),
    ],
    search_keys: &[
        ("↑↓", "escolher"),
        ("Enter", "selecionar"),
        ("Esc", "cancelar"),
    ],
    alarm_keys: &[
        ("n", "novo"),
        ("Espaço", "liga/desliga"),
        ("r", "diário"),
        ("d", "apagar"),
        ("Esc", "fechar"),
    ],
    input_keys: &[("Enter", "salvar"), ("Esc", "cancelar")],
    ringing_keys: &[("qualquer tecla", "parar"), ("z", "soneca 5 min")],
    theme_keys: &[
        ("↑↓", "experimentar"),
        ("Enter", "usar"),
        ("Esc", "cancelar"),
    ],
    calendar_keys: &[
        ("e", "editar endereço"),
        ("+ -", "lembrete"),
        ("r", "atualizar"),
        ("d", "desconectar"),
        ("Esc", "fechar"),
    ],
    mac_calendar_keys: &[
        ("+ -", "lembrete"),
        ("r", "atualizar"),
        ("o", "privacidade"),
        ("a", "contas"),
        ("d", "desconectar"),
        ("Esc", "fechar"),
    ],
    choose_keys: &[("↑↓", "escolher"), ("Enter", "conectar"), ("Esc", "fechar")],
    help_keys: &[
        ("c  /", "adicionar uma cidade (ela substitui a hora local)"),
        ("←  →", "trocar de cidade (também Tab, 1-9)"),
        ("x", "fechar a cidade; fechando a última volta a hora local"),
        ("m", "mostrar ou esconder o mapa nesta aba"),
        ("a", "alarmes, no fuso desta aba"),
        ("v", "estilo do mapa: ASCII, braille, blocos"),
        ("t", "tema do relógio: 50 temas de editores"),
        (
            "w  W",
            "mostrar ou esconder a previsão; W define sua cidade",
        ),
        (
            "g",
            "agenda: Calendário do Mac ou iCal, lembrete 5 min antes",
        ),
        ("s", "mostrar ou esconder os segundos"),
        ("q  Ctrl+C", "sair"),
        ("", ""),
        ("", "Quando um alarme toca, a tela pisca:"),
        ("", "qualquer tecla para, z adia 5 minutos."),
    ],
    weekdays: [
        "segunda-feira",
        "terça-feira",
        "quarta-feira",
        "quinta-feira",
        "sexta-feira",
        "sábado",
        "domingo",
    ],
    months: [
        "janeiro",
        "fevereiro",
        "março",
        "abril",
        "maio",
        "junho",
        "julho",
        "agosto",
        "setembro",
        "outubro",
        "novembro",
        "dezembro",
    ],
};

impl Lang {
    /// Portuguese when the locale variables ask for it, English otherwise.
    pub fn from_env() -> Self {
        let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty())
            .unwrap_or_default();
        Self::from_locale(&locale)
    }

    fn from_locale(locale: &str) -> Self {
        if locale.to_ascii_lowercase().starts_with("pt") {
            Self::Pt
        } else {
            Self::En
        }
    }

    pub fn text(self) -> &'static Text {
        match self {
            Self::En => &EN,
            Self::Pt => &PT,
        }
    }

    /// "Friday, September 25, 2026" or "sexta-feira, 25 de setembro de 2026".
    pub fn long_date(self, date: NaiveDate) -> String {
        let (weekday, month) = self.names(date);
        match self {
            Self::En => format!("{weekday}, {month} {}, {}", date.day(), date.year()),
            Self::Pt => format!("{weekday}, {} de {month} de {}", date.day(), date.year()),
        }
    }

    /// "Fri" or "sex".
    pub fn short_weekday(self, date: NaiveDate) -> String {
        self.names(date).0.chars().take(3).collect()
    }

    /// "Fri, Sep 25" or "sex, 25 set".
    pub fn short_date(self, date: NaiveDate) -> String {
        let (weekday, month) = self.names(date);
        let short = |name: &str| name.chars().take(3).collect::<String>();
        match self {
            Self::En => format!("{}, {} {}", short(weekday), short(month), date.day()),
            Self::Pt => format!("{}, {} {}", short(weekday), date.day(), short(month)),
        }
    }

    fn names(self, date: NaiveDate) -> (&'static str, &'static str) {
        let text = self.text();
        (
            text.weekdays[date.weekday().num_days_from_monday() as usize],
            text.months[date.month0() as usize],
        )
    }

    /// Time left until something: "2d 3h", "3h 05m", "12m 30s", "45s" ("min" in Portuguese).
    pub fn countdown(self, left: Duration) -> String {
        let seconds = left.num_seconds().max(0);
        let (days, hours) = (seconds / 86_400, seconds % 86_400 / 3600);
        let (minutes, seconds) = (seconds % 3600 / 60, seconds % 60);
        let min = match self {
            Self::En => "m",
            Self::Pt => "min",
        };
        if days > 0 {
            format!("{days}d {hours}h")
        } else if hours > 0 {
            format!("{hours}h {minutes:02}{min}")
        } else if minutes > 0 {
            format!("{minutes}{min} {seconds:02}s")
        } else {
            format!("{seconds}s")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_locale() {
        assert_eq!(Lang::from_locale("pt_BR.UTF-8"), Lang::Pt);
        assert_eq!(Lang::from_locale("PT_pt"), Lang::Pt);
        assert_eq!(Lang::from_locale("en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }

    #[test]
    fn formats_dates() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        assert_eq!(Lang::En.long_date(date), "Saturday, September 26, 2026");
        assert_eq!(Lang::Pt.long_date(date), "sábado, 26 de setembro de 2026");
        assert_eq!(Lang::En.short_date(date), "Sat, Sep 26");
        assert_eq!(Lang::Pt.short_date(date), "sáb, 26 set");
    }

    #[test]
    fn formats_countdowns() {
        let s = Duration::seconds;
        assert_eq!(Lang::En.countdown(s(45)), "45s");
        assert_eq!(Lang::En.countdown(s(750)), "12m 30s");
        assert_eq!(Lang::Pt.countdown(s(750)), "12min 30s");
        assert_eq!(Lang::En.countdown(s(3 * 3600 + 5 * 60)), "3h 05m");
        assert_eq!(Lang::En.countdown(s(2 * 86_400 + 3 * 3600)), "2d 3h");
        assert_eq!(Lang::En.countdown(s(-5)), "0s");
    }

    #[test]
    fn both_languages_have_the_same_keys() {
        let (en, pt) = (Lang::En.text(), Lang::Pt.text());
        let keys = |list: Keys| list.iter().map(|(key, _)| *key).collect::<Vec<_>>();
        assert_eq!(keys(en.clock_keys), keys(pt.clock_keys));
        assert_eq!(keys(en.help_keys), keys(pt.help_keys));
        assert_eq!(en.alarm_keys.len(), pt.alarm_keys.len());
    }
}
