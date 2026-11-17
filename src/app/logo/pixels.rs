pub(crate) const SIDE: usize = 16;

pub(crate) const ROWS: [&str; SIDE] = [
    "................",
    "......KKKK......",
    "....KKKKKKKK....",
    "...KKKKKKKKKK...",
    "...KWKKKKKKWK...",
    "..GRRRKKKKRRRG..",
    ".GGRRRKNNKRRRGG.",
    ".GRRRRRKKRRRRRG.",
    ".KRRRRRRRRRRRRK.",
    ".KRRRRRRRRRRRRK.",
    ".KWRRRRRRRRRRWK.",
    ".KKRRRRRRRRRRKK.",
    "..KKDRRRRRRDKK..",
    ".....DWWWWD.....",
    ".....O.KK.O.....",
    "....OO.KK.OO....",
];

const BLACK: u32 = 0x111111;
const WHITE: u32 = 0xffffff;
const BILL: u32 = 0x3b3d44;
const ROSE: u32 = 0xe5584a;
const ROSE_SHADE: u32 = 0xc8463c;
const SHOULDER_GREY: u32 = 0x8e99a7;
const LEGS: u32 = 0xa4705c;

pub(crate) fn rgb(pixel: u8) -> Option<u32> {
    match pixel {
        b'K' => Some(BLACK),
        b'W' => Some(WHITE),
        b'N' => Some(BILL),
        b'R' => Some(ROSE),
        b'D' => Some(ROSE_SHADE),
        b'G' => Some(SHOULDER_GREY),
        b'O' => Some(LEGS),
        _ => None,
    }
}
