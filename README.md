# meridian

A terminal clock that fits any window. Big digits, the time of any city with a
world map showing where it is (and where it is night), and alarms that make the
screen blink. Built for tiling window managers: resize the pane and the layout
follows.

```text
                                         Tokyo · Japan

       █████████      ███            █████████   ███   ███         █████████   █████████
       ███▀▀▀███   ▄▄▄███      ▄▄▄   ███▀▀▀███   ███   ███   ▄▄▄   ███▀▀▀███   ███▀▀▀▀▀▀
       ███   ███   ██████      ███   ███   ███   ███   ███   ███   ███   ███   ███
       ███   ███      ███            ███   ███   █████████         ███   ███   █████████
       ███   ███      ███      ▄▄▄   ███   ███   ▀▀▀▀▀▀███   ▄▄▄   ███   ███   ▀▀▀▀▀▀███
       ███   ███      ███      ███   ███   ███         ███   ███   ███   ███         ███
       █████████   █████████         █████████         ███         █████████   █████████
       ▀▀▀▀▀▀▀▀▀   ▀▀▀▀▀▀▀▀▀         ▀▀▀▀▀▀▀▀▀         ▀▀▀         ▀▀▀▀▀▀▀▀▀   ▀▀▀▀▀▀▀▀▀

                         Saturday, September 26, 2026 · JST · UTC+09:00
                .:....::.+++++++##########:.       ..:      . .:.      .+::::.     .:.
::..:+++++++++++#++#:+#+#+:++:.  +######:: . .     :+++::...:+++++#+##############+####+#++++++
   .:##++++############:...+#++.  :+:    .:.  : .+##:+###############################++++++++:.
             :############+###++:.          .::#+###################################::  ::
              .#########+#+#:..              +#+::::+#++++##:+#################+##..:..
               .:+########:.               ..+#####+:++++######################: .::●  Tokyo
                  .+##... ....             +#############+#####:::####++####++:.
                     .::::..:::.         . +################+.    .++.  .++#.  :.
                       .  :###☼##+.         .::::+#########+.       ..  .:+..++....         .
                          +#########++.           :#######.               ::::.:. .:++:...
         .                 :+########:            :#######.::                  .++#+:+      .
                            .######:.              +####:  +.                :#########+   .
                            +###+:.                 :+:.                      ++::::###:     ..
                           .##+                                                      ..    .:..
                            :+ .     .                           .
 c city  l local  a alarms  m map  v style  s seconds  ? help  q quit
```

<details>
<summary>Braille map style (<code>v</code> or <code>--map-style braille</code>)</summary>

```text
                                       São Paulo · Brazil

          ███      █████████         █████████   ███   ███         █████████   █████████
       ▄▄▄███      ▀▀▀▀▀▀███   ▄▄▄   ███▀▀▀███   ███   ███   ▄▄▄   ███▀▀▀███   ███▀▀▀▀▀▀
       ██████            ███   ███   ███   ███   ███   ███   ███   ███   ███   ███
          ███      █████████         ███   ███   █████████         ███   ███   █████████
          ███      ▀▀▀▀▀▀███   ▄▄▄   ███   ███   ▀▀▀▀▀▀███   ▄▄▄   ███   ███   ▀▀▀▀▀▀███
          ███            ███   ███   ███   ███         ███   ███   ███   ███         ███
       █████████   █████████         █████████         ███         █████████   █████████
       ▀▀▀▀▀▀▀▀▀   ▀▀▀▀▀▀▀▀▀         ▀▀▀▀▀▀▀▀▀         ▀▀▀         ▀▀▀▀▀▀▀▀▀   ▀▀▀▀▀▀▀▀▀

                             Friday, September 25, 2026 · UTC-03:00
                ⣀⣀⢀⡀⢀⣄⠔⣒⠢⣴⡾⠟⠻⠭⠶⠶⣶⣶⣶⣶⣿⣿⣿⣿⣿⣶⡒⠂       ⠠⠔⠄      ⠂ ⠐⣀⡀      ⢐⣒⣀⣀⣠⣀⡀     ⢀⣀⣀
⢄⣀⡀⠠⡤⣤⣦⣦⣤⣤⣤⣤⣤⣤⣬⡭⢽⣶⣖⣞⣀⣩⣚⣥⣟⡿⠛⠒⡶⡄⣀  ⢹⣿⣿⣿⣿⠿⠿⠝⢚⢀⡀       ⣠⣤⣤⣦⣤⡤⢄⢀⣀⣀⣐⣧⣠⣤⣶⣧⣶⣾⣽⣿⣿⣿⣿⣿⣷⣾⣿⣿⣶⣿⣷⣶⣶⣶⣷⣶⣦⣶⣦⣤⣤⣤⣤⣤
   ⠐⠰⢿⠿⠛⠛⠛⠛⠝⣿⣿⣿⣿⣿⣽⣟⣿⣿⢿⣟⣁⠈⠉⠁⢶⣮⣉⣇⡀  ⠙⠛⠋    ⠈⠉   ⣀  ⢶⠻⣿⠍⣪⣽⣿⣯⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠿⠛⠛⠻⢛⣩⡟⠛⠛⠋⠁
             ⠉⢟⣿⣽⣿⣿⣿⣿⣿⣿⡿⡿⣷⣬⢿⣿⣿⡻⠛⠖⠄           ⠊⠚⣳⣴⣶⣷⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣾⣿⣿⣿⣿⣿⣿⣷⠖⠆  ⠈⠋
              ⠰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣷⣾⡿⠛⠉⠉              ⣶⣶⡟⠋⠩⠙⡨⠽⡟⢻⣥⣬⣬⣽⣿⡄⢹⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠻⢿⡟⠉⠁⡰⠂
               ⠈⠙⠻⣿⣿⣿⣿⠿⠿⠿⢿⠋                ⢀⢀⣶⣶⣿⣿⣿⣧⣤⣤⣬⣤⣤⢤⣿⣿⣿⡿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣟⣿⡆ ⠑⠊⠉⠁
     ⠠            ⠈⠹⣿⣯ ⢀⡄ ⠁ ⢀⡀             ⣼⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡝⢿⣿⣿⣶⡾⠉⠉⠙⠿⣿⣿⣿⠟⠛⢿⣿⣿⡿⡛⠛⠋⠂
                     ⠈⠉⠙⠲⢆⢀⢀⡀⣀⣀⡀           ⠿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⣛⣋⠁     ⢻⣏   ⠈⠹⠻⡿⠆ ⢀⠉
                          ⢀⣿⣿⣿☼⣿⣶⣶          ⠈⠉⠉⠉⠉⠛⣿⣿⣿⣿⣿⣿⢿⣿⣿⠟⠁       ⠈   ⠑⠢⣲ ⢀⣴⡖⠈⠉⡀
                          ⠻⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣦⠄           ⠙⣿⣿⣿⣿⣽⣾⣿⠁               ⠉⠣⠬⠉ ⠁⡀ ⠄⠘⠷⠶⢄
                           ⠙⠻⣿⣿⣿⣿⣿⣿⣿⣿⠁            ⠰⣿⣿⣿⣿⣿⣯⠿ ⣤⡖                  ⢀⣴⣴⣾⣦⣄⣦        ⠠
                            ⢀⣿⣿⣿⣿⣿⡟●⠁ São Paulo    ⠹⣿⣿⣿⣿⠛  ⠛                 ⢰⣿⣿⣿⣿⣿⡿⣿⣿⣿⣦
                            ⣸⣿⣿⣿⡟⠋                  ⠛⠛⠛⠁                      ⠛⠛⠋⠉⠉⠻⢿⣿⡿⠋     ⢀
                            ⣽⣿⠋                                                       ⠂    ⠠⠔⠊
                           ⠐⠚⠧ ⠐
 c city  l local  a alarms  m map  v style  s seconds  ? help  q quit
```

</details>

On a real terminal the land is green by day, yellow at dusk and blue at night,
the city is a red dot and `☼` marks where the Sun is overhead.

## Features

- **Your computer's time or any city's.** Press `c` and type a city. Accents
  are optional and local names work: `sao paulo`, `Tóquio`, `Nova Iorque`,
  `Munique`. Add a region or country after a comma to tell homonyms apart:
  `Portland, Maine`, `Paris, US`. The list shows each city's current time.
  34 000 cities are built in, so no network is needed.
- **ASCII world map.** Picking a city brings up the map with a marker on it
  and the night side shaded. `v` switches between ASCII, braille and block
  drawings; `m` shows or hides the map, in local time too.
- **Responsive.** The digits grow and shrink with the window. The map goes
  under or beside the clock, whichever uses the pane best, and steps aside when
  there is no room. The time stays readable down to a 20×5 pane.
- **Blinking alarms.** `a` opens the alarm list. Type `07:30`, `7h30`,
  `7:30pm` or a delay such as `+10m`, optionally followed by a label. When an
  alarm rings the whole screen flashes red and the terminal bell rings, which
  tiling window managers show as an urgent window. Any key stops it; `z`
  snoozes for 5 minutes. Alarms can repeat daily.
- **English or Portuguese**, picked from `$LANG` (or `--lang`).

## Install

```sh
cargo install --git https://github.com/victorlcampos/meridian
```

Needs Rust 1.88 or newer ([rustup.rs](https://rustup.rs)).

## Usage

```sh
meridian                                    # the computer's time
meridian --city Tokyo                       # a city's time, with the world map
meridian -c "São Paulo" --map-style braille
meridian --alarm 07:30 --alarm "+25m Tea"   # alarms from the command line
meridian --map --color green --no-seconds   # map in local time, green digits
```

Run `meridian --help` for every option.

### Keys

| Key | Action |
| --- | --- |
| `c` or `/` | Pick a city |
| `l` | Back to the computer's time |
| `a` | Alarms |
| `m` | Show or hide the world map |
| `v` | Map style: ASCII, braille, blocks |
| `s` | Show or hide seconds |
| `?` | Help |
| `q` or `Ctrl+C` | Quit |

In the city list: type to search, `↑`/`↓` to choose, `Enter` to select, `Esc`
to cancel.

In the alarm list: `n` new alarm, `Space` switch on or off, `r` repeat daily,
`d` delete, `Esc` close.

While an alarm rings: any key stops it, `z` snoozes for 5 minutes.

### Alarm times

| You type | It rings |
| --- | --- |
| `07:30`, `7:30`, `7h30`, `730` | Next time the clock shows 07:30 |
| `7:30pm`, `19:30` | Next time the clock shows 19:30 |
| `+10`, `+10m` | In 10 minutes |
| `+1h30`, `+90s` | In 1 h 30 min, in 90 seconds |
| `07:30 Stand-up` | At 07:30, labeled "Stand-up" |

An alarm follows the clock on screen when it is created: set while looking at
Tokyo, it rings at that time in Tokyo. Alarms live while meridian runs.

## Tiling window managers

Each instance is independent, so a pane per city works well:

```sh
# i3 / sway
exec alacritty --class meridian -e meridian --city Tokyo
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

The files in `assets/` are generated by `python3 tools/gen_assets.py`, which
downloads the sources and rebuilds them.

## Em português

Relógio de terminal responsivo, feito para tiling window managers. Mostra a
hora do computador ou de qualquer cidade: tecle `c` e digite o nome, com ou
sem acento, inclusive em português (`Tóquio`, `Nova Iorque`, `Munique`). Ao
escolher a cidade aparece o mapa-múndi em ASCII com a cidade marcada e o lado
da noite sombreado. Alarmes (`a`) aceitam `07:30`, `7h30` ou `+10m` com um
nome opcional; quando tocam, a tela pisca até você apertar uma tecla (`z` adia
5 minutos). A interface fica em português quando `$LANG` começa com `pt`.

## License

[MIT](LICENSE)
