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
    pub city: &'static str,
    pub city_prompt: &'static str,
    pub no_match: &'static str,
    pub alarms: &'static str,
    pub no_alarms: &'static str,
    pub new_alarm: &'static str,
    pub alarm_example: &'static str,
    pub invalid_time: &'static str,
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
    pub help_keys: Keys,
    weekdays: [&'static str; 7],
    months: [&'static str; 12],
}

const EN: Text = Text {
    local_time: "Local time",
    city: "City",
    city_prompt: "Type a city: Tokyo, São Paulo, Portland, Maine…",
    no_match: "No city found",
    alarms: "Alarms",
    no_alarms: "No alarms yet. Press n to add one.",
    new_alarm: "New alarm",
    alarm_example: "07:30, 7h30, 7:30pm or +10m, then an optional label",
    invalid_time: "Invalid time. Try 07:30, 7:30pm or +10m",
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
        ("l", "local"),
        ("a", "alarms"),
        ("m", "map"),
        ("v", "style"),
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
    help_keys: &[
        ("c  /", "pick a city"),
        ("l", "back to the computer's time"),
        ("a", "alarms"),
        ("m", "show or hide the world map"),
        ("v", "map style: ASCII, braille, blocks"),
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
    city: "Cidade",
    city_prompt: "Digite uma cidade: Tóquio, São Paulo, Porto, Portugal…",
    no_match: "Nenhuma cidade encontrada",
    alarms: "Alarmes",
    no_alarms: "Nenhum alarme. Tecle n para criar um.",
    new_alarm: "Novo alarme",
    alarm_example: "07:30, 7h30 ou +10m, e um nome opcional",
    invalid_time: "Horário inválido. Tente 07:30, 7h30 ou +10m",
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
        ("l", "local"),
        ("a", "alarmes"),
        ("m", "mapa"),
        ("v", "estilo"),
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
    help_keys: &[
        ("c  /", "escolher uma cidade"),
        ("l", "voltar para a hora do computador"),
        ("a", "alarmes"),
        ("m", "mostrar ou esconder o mapa-múndi"),
        ("v", "estilo do mapa: ASCII, braille, blocos"),
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
