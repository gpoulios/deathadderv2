use rgb::RGB8;
use libdeathadder::core::{rgb_from_hex, Config};
use libdeathadder::v2::set_color;

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

    let (color, wheel_color) = match args.len() {
        ..=1 => {
            match Config::load() {
                Some(cfg) => (cfg.color, cfg.wheel_color),
                None => panic!("failed to load configuration; please specify \
                    arguments manually")
            }
        },
        2 => (parse_arg(args[1].as_ref()), None),
        3 => (parse_arg(args[1].as_ref()), Some(parse_arg(args[2].as_ref()))),
        _ => panic!("usage: {} [(body) color] [wheel color]", args[0])
    };

    match set_color(color, wheel_color) {
        Ok(msg) => println!("{}", msg),
        Err(e) => panic!("Failed to set color(s): {}", e)
    }
}
