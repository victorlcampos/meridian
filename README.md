# meridian

A terminal clock that fits any window. Big digits, a tab per city with a world
map showing where it is (and where it is night), the weather for the next
hours, your next calendar event, 50 color themes and alarms that make the
screen blink. Built for tiling window managers: resize the pane and the layout
follows. Everything you set is saved as you go.

```text
 Tokyo 01:04 │ London 17:04 │ New York City 12:04 │ Sydney 02:04
                                         Tokyo · Japan

                     ██████    ██        ██████  ██  ██      ██████  ██████
                     ██  ██  ████    ██  ██  ██  ██  ██  ██  ██  ██  ██
                     ██  ██    ██        ██  ██  ██████      ██  ██  ██████
                     ██  ██    ██    ██  ██  ██      ██  ██  ██  ██      ██
                     ██████  ██████      ██████      ██      ██████  ██████

                         Saturday, September 26, 2026 · JST · UTC+09:00
                         Next event 01:39 Sprint planning · in 35m 00s
                                 Drizzle 21°C · rain 97% at 05h
                       02h 21° 20%  03h 21° 64%  05h 20° 97%  06h 20° 0%
               . :..:::+.::++#++##########+.       ...      .. .::      .::+::      .
:.  :+++++::::+#+###:+++++:+:+:  +#######+.        .:+++:::..:+:+:+:+++###########+++++#++::::+:
 . .###+++#############+.:.+##:.  :++.    .:. .. :+##:####+##############################++##++:
.    .      .:############+###+++            ::+:+###################################::  :+.
              .#########+####+:. .           .+##+++###+++###:#####################+::.
               :###########:                 :##+#+:.::::+######################..:.:●  Tokyo
                 .:###.. .:                .+############+###+#+++#############+:
                   .:+:++. ... .           :##############+###+:  .:##:..+##+:  :
                         : :###+::          +###############+.      :.   .:::..:.:
                          :####☼###::.            :########+.             :+.:#:+. :::...
                          .###########:            +######+  .             . ..   .::+::.. .
                           .:########:             +######::#.                .::####+#.    .
                            .######:               .####+. ..                 +#########+
                            :###+:                  .::.                      .::. .:+##:     :.
                           .+#:                                                       ..    .:.
                            :+.       .                                                   .
 c city  ←→ tabs  x close  m map  v map style  t theme  w weather  g calendar  a alarms
```

<details>
<summary>Local time with its map open, in braille (<code>m</code>, then <code>v</code>), marking your city (<code>W</code>)</summary>

```text
                                 Local time · America/Sao_Paulo

                       ██    ██████      ██████  ██  ██      ██████  ██████
                     ████        ██  ██  ██  ██  ██  ██  ██  ██  ██  ██
                       ██    ██████      ██  ██  ██████      ██  ██  ██████
                       ██        ██  ██  ██  ██      ██  ██  ██  ██      ██
                     ██████  ██████      ██████      ██      ██████  ██████

                             Friday, September 25, 2026 · UTC-03:00
                         Next event 13:39 Sprint planning · in 35m 00s
                         Rio de Janeiro: Drizzle 21°C · rain 97% at 17h
                       14h 21° 20%  15h 21° 64%  17h 20° 97%  18h 20° 0%
               ⢀⣀ ⢀ ⡠ ⢐⠦⣶⣖⣮⠽⠛⠻⣵⣶⣶⣶⣶⣾⣿⣿⣿⣿⣿⣷⠖⠒       ⠤⠤⠄ ⠠     ⠂⠐⠂⠐⢂⡀  ⠂  ⠐⠒⠄⣤⡀⡀
⡀  ⢀⣀⣤⣤⣤⣄⣀⣀⣀⣀⣀⣐⣒⣒⡚⠷⣶⠔⠒⠲⣄⡚⣷⠧⠦⢤⣄   ⠹⣿⣿⣿⣿⣿⣿⡿⠛⠉         ⢀⣀⣄⣤⣄⣀⡀⢀⡀⢀⣒⣩⣀⣀⢴⡦⡤⣴⣲⣿⢿⣿⣿⣾⣿⣷⣶⣶⣶⣶⣦⣤⣤⣴⣦⣤⣤⣤⣄⣀⣀⣀⣀⣀
⠈⠉⠁ ⠥⣿⣿⢿⠿⠿⠿⣿⣿⣿⣷⣷⣾⣿⣻⣿⣿⣿⡯⠛⠋⠂⠠⢤⣝⡛⢎⠁  ⠹⠿⠟⠉⠁  ⠈⠒⠋     ⢠⣴⣾⣗⠫⠽⢟⣾⣦⣷⣷⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠿⠿⡿⠿⠿⠿⠋
      ⠁⠁   ⠈⠨⠻⣿⣿⣿⣿⣿⣿⣷⣟⣿⣷⣶⣤⡐⣜⣿⡿⣿⣷⣄            ⠠⠜⣦⡀⣀⣢⣿⣥⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⣟⣿⣿⣿⣿⣿⣿⣿⣏⡅   ⢰⠟
               ⣿⣿⣿⣿⣿⣿⣻⣷⣯⡍⣙⡻⢿⣿⠼⠆⠅⠈⠐           ⢀⣀⣰⣿⢿⠿⡿⢿⣿⣿⡿⠻⠟⢿⣿⡟⢛⣿⣿⣿⣿⣿⣿⣿⣿⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠟⢁⡁
               ⠻⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡟⠁                ⠸⢿⣟⣁⣀⣀⠨⠁⠹⠉⠻⢾⢿⣶⣿⣿⣄⣸⣿⣿⣿⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣯⠍⢙⠅⣀⣀⠔
                ⠈⠫⡿⣿⣿⣿⠟⠛⠛⠛⡇                ⠄⢀⣼⣿⣿⣿⣿⣿⣷⣶⣼⣶⣶⣖⢺⣿⣿⣿⡻⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⠇
                   ⠈⢿⣿⣀⣀⡄⠐⠒⡠  ⡀            ⢰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⡹⣿⣿⣷⣾⠖  ⠙⢻⣿⣿⡿⠛⠹⣿⣿⣿⡟⠛⠋⠁⡃
                      ⠁⠙⠓⣖  ⣠⣀⣀⡀           ⠸⢿⣿⣿⣿⣿⢿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣜⣛⡉     ⠈⢿⠇    ⠛⠻⡿   ⢓
                           ⣹⣿⣿⣿⣿⣷⣦⡄         ⠈⠙⠛⠛⠉⠙⢻⣿⣿⣿⣿⣿⢿⣿⣿⣿⠟⠁            ⢕⢆ ⣀⣤⡂⠈⠁
                          ⢸⣿⣿⣿⣿☼⣿⣿⣿⣶⣤⣤⡀           ⠘⢿⣿⣿⣿⣟⣧⣿⡏⠁              ⠈⠳⢔⣙⢛⠑⠢  ⠑⠰⣦⣤ ⠂
                           ⠹⣿⣿⣿⣿⣿⣿⣿⣿⣿⡟⠁            ⣸⣿⣿⣿⣿⣿⣹⣷ ⢀⡄                 ⠁⠁⢀⣠⣤⠄⢈⡅⠁
                             ⣹⣿⣿⣿⣿⣿⣿●⠃ Rio de Janeiro ⣿⣿⣿⡏ ⢨⡿                 ⢀⣤⡼⣿⣿⣿⣿⣾⣿⣤⡀
                             ⣿⣿⣿⣿⣿⠟                 ⠻⣿⣿⣿⠇                     ⠸⣿⣿⣿⡿⣿⣿⣿⣿⣿⡿
                            ⢰⣿⣿⡿⠏⠁                   ⠉⠉                       ⠈⠉⠈   ⠉⠻⠿⠟⠁     ⢰⠄
                            ⣽⣿⠋                                                       ⠈⠁     ⠊⠁
                            ⠙⠑⠤
 c city  m map  v map style  t theme  w weather  g calendar  a alarms  s seconds  ? help
```

</details>

On a real terminal the land is green by day and blue at night, the cities are
dots and `☼` marks where the Sun is overhead, all in the colors of the theme
you pick.

## Features

- **A tab per city.** meridian starts on your computer's time. Press `c` and
  type a city to add it: from then on the tabs are your cities, and closing
  the last one brings local time back. Accents are optional and local names
  work: `sao paulo`, `Tóquio`, `Nova Iorque`, `Munique`. Add a region or
  country after a comma to tell homonyms apart: `Portland, Maine`,
  `Paris, US`. `←` `→` switch cities, `x` closes one, and the tab bar shows
  the time in every city. 34 000 cities are built in.
- **ASCII world map.** A city tab opens with the map, the city marked and the
  night side shaded. Local time can show it too, with your city marked. `m` shows or hides the map of the tab on screen; `v` switches
  between ASCII, braille and block drawings.
- **50 themes.** `t` lists color themes after well-known editor themes:
  Tokyo Night, Dracula, Catppuccin, Gruvbox, Nord, Solarized and more. Moving
  through the list shows each one at once; type to filter.
- **Weather.** City tabs show the weather now, when rain is likely and the
  next hours' temperature and chance of rain. In local time, `w` asks for
  your city first; after that `w` shows or hides the weather and `W`
  changes the city.
- **Calendar.** `g` connects the Mac Calendar (every account in the macOS
  Calendar app, Google included) or any iCal feed. The next event shows under
  the clock, and a reminder rings 5 minutes before it.
- **Blinking alarms.** `a` opens the alarm list. Type `07:30`, `7h30`,
  `7:30pm` or a delay such as `+10m`, optionally followed by a label. When an
  alarm rings the whole screen flashes and the terminal bell rings, which
  tiling window managers show as an urgent window. Any key stops it; `z`
  snoozes for 5 minutes. Alarms can repeat daily.
- **Responsive.** The digits grow and shrink with the window. The map goes
  under or beside the clock, whichever uses the pane best, and steps aside when
  there is no room. The time stays readable down to a 20×5 pane.
- **Saved as you go.** Tabs, theme, map, alarms and calendar are written to
  disk on every change, so they come back even after a power cut.
- **English or Portuguese**, picked from `$LANG` (or `--lang`).

## Install

```sh
cargo install --git https://github.com/victorlcampos/meridian
```

Needs Rust 1.88 or newer ([rustup.rs](https://rustup.rs)).

On macOS, sign the installed binary once more so macOS can read the reason
meridian gives when it asks for your calendars (repeat after each update):

```sh
codesign --force --sign - --identifier io.github.victorlcampos.meridian ~/.cargo/bin/meridian
```

## Usage

```sh
meridian                                    # the computer's time
meridian --city Tokyo                       # a city's time, with the world map
meridian -c Tokyo -c London -c "São Paulo"  # a tab per city
meridian --theme "tokyo night"              # see --list-themes
meridian --alarm 07:30 --alarm "+25m Tea"   # alarms from the command line
meridian --calendar "https://calendar.google.com/calendar/ical/…/basic.ics"
```

Command-line options work as if you did the same in the app, so they are
saved too. Run `meridian --help` for every option.

### Keys

| Key | Action |
| --- | --- |
| `c` or `/` | Add a city (it takes the place of local time) |
| `←` `→` | Switch cities (also `Tab`, `Shift+Tab`, `1`-`9`) |
| `x` | Close the city; closing the last one brings local time back |
| `m` | Show or hide the map in this tab |
| `v` | Map style: ASCII, braille, blocks (brings a hidden map up first) |
| `t` | Clock theme |
| `w` | Show or hide the weather (in local time, pick your city first) |
| `W` | Change your city, for the weather in local time |
| `g` | Calendar |
| `a` | Alarms |
| `s` | Show or hide seconds |
| `?` | Help |
| `q` or `Ctrl+C` | Quit |

In the city list: type to search, `↑`/`↓` to choose, `Enter` to open, `Esc`
to cancel. Picking a city that already has a tab goes to that tab.

In the theme list: type to filter, `↑`/`↓` to try, `Enter` to keep, `Esc` to
go back to the theme you had.

In the alarm list: `n` new alarm, `Space` switch on or off, `r` repeat daily,
`d` delete, `Esc` close.

In the calendar: pick the Mac Calendar or an iCal address; then `+` `-`
change the reminder minutes (0 turns it off), `r` refreshes, `d` disconnects,
`e` edits an iCal address, and for the Mac Calendar `o` opens the privacy
setting and `a` the Internet Accounts.

While an alarm or reminder rings: any key stops it, `z` snoozes for 5 minutes.

### Alarm times

| You type | It rings |
| --- | --- |
| `07:30`, `7:30`, `7h30`, `730` | Next time the clock shows 07:30 |
| `7:30pm`, `19:30` | Next time the clock shows 19:30 |
| `+10`, `+10m` | In 10 minutes |
| `+1h30`, `+90s` | In 1 h 30 min, in 90 seconds |
| `07:30 Stand-up` | At 07:30, labeled "Stand-up" |

An alarm follows the tab on screen when it is created: set in the Tokyo tab,
it rings at that time in Tokyo. An alarm that came due while the computer was
off rings as soon as meridian opens.

### Themes

`Terminal` (the default) keeps your terminal's own colors. The other 50:
Tokyo Night, Tokyo Night Storm, Tokyo Night Moon, Tokyo Night Day, Dracula, One Dark, One Light, Monokai, Monokai Pro, Nord, Gruvbox Dark, Gruvbox Light, Solarized Dark, Solarized Light, Catppuccin Mocha, Catppuccin Macchiato, Catppuccin Frappé, Catppuccin Latte, Rosé Pine, Rosé Pine Moon, Rosé Pine Dawn, Kanagawa, Kanagawa Dragon, Everforest Dark, Everforest Light, Ayu Dark, Ayu Mirage, Ayu Light, GitHub Dark, GitHub Light, VS Code Dark+, VS Code Light+, Darcula, Material, Palenight, Night Owl, Cobalt2, Shades of Purple, SynthWave '84, Oceanic Next, Panda, Horizon, Zenburn, Tomorrow Night, Tomorrow Night Blue, Nightfox, Oxocarbon, Vitesse Dark, Xcode Dark, Mariana.

Terminals that do not advertise 24-bit color (`COLORTERM=truecolor`) get the
nearest colors of the 256-color palette.

## Calendar

Press `g` and pick where events come from. Either way, recurring events are
followed, all-day, cancelled and declined events are left out, and the
reminder rings 5 minutes before each event by default; `+` and `-` change it,
0 turns it off (or `--reminder <minutes>`).

### Mac Calendar (macOS)

meridian reads the events of the macOS Calendar app, so any account it syncs
works, Google Workspace included, with no address or Google Cloud setup:

1. Add your account in System Settings › Internet Accounts (for Google, turn
   on *Calendars*). The Calendar app should show your events.
2. In meridian press `g`, pick *Mac Calendar* and allow access when macOS
   asks ("meridian would like full access to your calendar").

macOS normally judges calendar access for the terminal a program runs in, and
refuses without asking when that terminal is built with the hardened runtime
and no calendar entitlement (cmux, for instance). So meridian reads the
calendar through a helper copy of itself that macOS treats as its own app,
with the reason embedded in the binary; that is what the `codesign` step
above binds. After an update macOS may ask again.

If you said no, allow your terminal in System Settings › Privacy & Security ›
Calendars (`o` opens it), then press `r`. The calendar is read every minute.

### iCal address

Any calendar that offers an iCal (.ics) address works, as does an `.ics`
file. For Google Calendar it is the *Secret address in iCal format* under
Settings › (your calendar) › *Integrate calendar* (organizations often hide
it; the Mac Calendar works then). Paste it after `g` › *iCal address*, or run
`meridian --calendar <address>`. It is read every 5 minutes.

The secret address gives read access to your calendar: it is kept only in the
state file below, which only your user can read, and never shown in full.

## Weather

Forecasts come from [Open-Meteo](https://open-meteo.com/) (free, no key),
refreshed every 30 minutes for each city tab and for your city (`W`), whose
weather local time shows.

## Saved state

Everything lives in `~/.config/meridian/state.json` (or
`$XDG_CONFIG_HOME/meridian/state.json`), rewritten on every change, all at
once, so a crash or a power cut leaves either the old file or the new one. A
file that cannot be read is kept as `state.json.broken` rather than lost.

## Tiling window managers

Copies of meridian running side by side share the state file: a city, theme or
alarm added in one shows up in the others within a second, while each keeps
its own tab on screen. For panes that should stay apart, give each its own
file:

```sh
# i3 / sway
exec alacritty --class meridian -e meridian --city Tokyo --state ~/.config/meridian/tokyo.json
```

The layout recalculates on every resize: full screen, half, a narrow column or
a short strip all get their own arrangement.

## Data

- Cities: [GeoNames](https://www.geonames.org/) `cities15000`, licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
- Land and lakes: [Natural Earth](https://www.naturalearthdata.com/) 1:50m,
  public domain, rasterized to a 1440×720 mask.
- Time zones: the IANA database, through
  [chrono-tz](https://crates.io/crates/chrono-tz).
- Weather: [Open-Meteo](https://open-meteo.com/), licensed
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

The files in `assets/` are generated by `python3 tools/gen_assets.py`, which
downloads the sources and rebuilds them.

## Em português

Relógio de terminal responsivo, feito para tiling window managers. Começa na
hora do computador; tecle `c` e digite uma cidade, com ou sem acento,
inclusive em português (`Tóquio`, `Nova Iorque`, `Munique`), para adicioná-la:
daí as abas passam a ser as suas cidades, `←` `→` trocam de cidade e `x` fecha
uma (fechar a última volta à hora local). Cada
aba de cidade abre com o mapa-múndi em ASCII, a cidade marcada, o lado da
noite sombreado e a previsão do tempo das próximas horas (`w` esconde); na hora
local o mapa abre e fecha com `m` e marca a sua cidade.
`v` troca o estilo do mapa e `t` o tema do relógio, entre 50 temas de editores
(Tokyo Night, Dracula, Catppuccin, Gruvbox…). Na hora local, `w` pergunta
sua cidade uma vez e passa a mostrar a previsão dela (`W` troca a cidade). `g` conecta o Calendário do Mac, com todas as contas do app
Calendário, Google Workspace incluída (adicione a conta em Ajustes do Sistema
› Contas de Internet e permita o acesso quando o macOS pedir), ou um endereço
iCal: o próximo evento aparece sob o relógio e um lembrete toca 5 minutos
antes. Alarmes (`a`) aceitam `07:30`, `7h30` ou `+10m` com um nome opcional;
quando tocam, a tela pisca até você apertar uma tecla (`z` adia 5 minutos).
Tudo é salvo a cada mudança em `~/.config/meridian/state.json` e volta mesmo
depois de o computador desligar. A interface fica em português quando `$LANG`
começa com `pt`.

## License

[MIT](LICENSE)
