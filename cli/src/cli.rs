use rgb::RGB8;
use librazer::cfg::Config;
use librazer::common::rgb_from_hex;
use librazer::device::{DeathAdderV2, RazerMouse};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let parse_arg = |input: &str| -> RGB8 {
        match rgb_from_hex(input) {
            Ok(rgb) => rgb,
            Err(e) => panic!("argument '{}' should be in the \
                form [0x/#]RGB[h] or [0x/#]RRGGBB[h] where R, G, and B are hex \
                digits: {}", input, e)
        }
    };

    let cfgopt = Config::load();

    let (logo_color, scroll_color) = match args.len() {
        ..=1 => {
            match cfgopt {
                Some(cfg) => (cfg.logo_color, cfg.scroll_color),
                None => panic!("failed to load configuration; please specify \
                    arguments manually")
            }
        },
        2..=3 => {
            let color = parse_arg(args[1].as_ref());
            (color, if args.len() == 3 {
                parse_arg(args[2].as_ref())
            } else {
                color
            })
        },
        _ => panic!("usage: {} [(body) color] [wheel color]", args[0])
    };

    let dav2 = DeathAdderV2::new().expect("failed to open device");

    _= dav2.set_logo_color(logo_color)
        .map_err(|e| panic!("failed to set logo color: {}", e))
        .and_then(|_| dav2.set_scroll_color(scroll_color))
        .map_err(|e| panic!("failed to set scroll color: {}", e));

    _ = Config {
        logo_color: logo_color,
        scroll_color: scroll_color,
        ..cfgopt.unwrap_or(Default::default())
    }.save().map_err(|e| panic!("failed to save config: {}", e));
}
