use rgb::RGB8;
use librazer::cfg::Config;
use librazer::common::rgb_from_hex;
use librazer::device::{DeathAdderV2, RazerMouse};

fn main() {
    // let dav2 = DeathAdderV2::new().expect("failed to open device");
    // println!("{}", dav2);
    // // dav2.set_dpi(10000, 10000);
    // // dav2.set_poll_rate(librazer::common::PollingRate::Hz250);
    // // println!("{:?}", dav2.get_dpi());
    // // println!("{:?}", dav2.get_poll_rate());
    // // dav2.set_dpi(20000, 20000);
    // // dav2.set_poll_rate(librazer::common::PollingRate::Hz1000);
    // // println!("{:?}", dav2.get_dpi());
    // // println!("{:?}", dav2.get_poll_rate());

    // let rgb1 = RGB8::from([0x00, 0xaa, 0xaa]);
    // let rgb2 = RGB8::from([0xaa, 0xaa, 0x00]);
    // // dav2.preview_static(rgb1, rgb2);
    // dav2.set_logo_color(rgb2);
    // dav2.set_scroll_color(rgb1);

    // // let init_brightness = dav2.get_logo_brightness().unwrap();
    // // println!("logo brightness: {:?}, scroll: {:?}", init_brightness, dav2.get_scroll_brightness());
    // // dav2.set_logo_brightness(30);
    // // dav2.set_scroll_brightness(30);
    // // println!("logo brightness: {:?}, scroll: {:?}", dav2.get_logo_brightness(), dav2.get_scroll_brightness());

    // return;

    let args: Vec<String> = std::env::args().collect();

    let parse_arg = |input: &str| -> RGB8 {
        match rgb_from_hex(input) {
            Ok(rgb) => rgb,
            Err(e) => panic!("argument '{}' should be in the \
                form [0x/#]RGB[h] or [0x/#]RRGGBB[h] where R, G, and B are hex \
                digits: {}", input, e)
        }
    };

    let (logo_color, scroll_color) = match args.len() {
        ..=1 => {
            match Config::load() {
                Some(cfg) => (cfg.color, cfg.scroll_color.or(Some(cfg.color)).unwrap()),
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
        color: logo_color,
        scroll_color: Some(scroll_color),
    }.save().map_err(|e| panic!("failed to save config: {}", e));
}
