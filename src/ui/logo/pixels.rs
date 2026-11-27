pub(crate) const SIDE: usize = 16;

pub(crate) const ROWS: [&str; SIDE] = [
    "................",
    "................",
    ".......KKK......",
    "......KKKKK.....",
    "......KKKWKOOOOK",
    "......WWWWWLLL..",
    ".....GWWWWW.....",
    "....GGGWWWW.....",
    "..GGGGGGWWWS....",
    ".DGGGGGGGWWS....",
    "DDDGGGGGWWWS....",
    "D..DDDWWWWS.....",
    "D.....SSSS......",
    "........R.R.....",
    ".......RR.RR....",
    "................",
];

const BLACK: u32 = 0x111111;
const WHITE: u32 = 0xffffff;
const PEARL_GREY: u32 = 0xc9d3dc;
const BELLY_LINE: u32 = 0x9aa7b3;
const WING_TIPS: u32 = 0x6f7c88;
const UPPER_BILL: u32 = 0xf26b1d;
const LOWER_BILL: u32 = 0xf7893a;
const LEGS: u32 = 0xe0452b;

pub(crate) fn rgb(pixel: u8) -> Option<u32> {
    match pixel {
        b'K' => Some(BLACK),
        b'W' => Some(WHITE),
        b'G' => Some(PEARL_GREY),
        b'S' => Some(BELLY_LINE),
        b'D' => Some(WING_TIPS),
        b'O' => Some(UPPER_BILL),
        b'L' => Some(LOWER_BILL),
        b'R' => Some(LEGS),
        _ => None,
    }
}
