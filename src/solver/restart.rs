/// Luby restart sequence (Luby, Sinclair, Zuckerman 1993);
/// `1,1,2,1,1,2,4,1,1,2,1,1,2,4,8,...`
pub fn luby(idx: usize) -> usize {
    let mut size = 1;
    let mut seq = 0;
    let mut x = idx;

    while size < x + 1 {
        seq += 1;
        size = 2 * size + 1;
    }

    while size - 1 != x {
        size = (size - 1) >> 1;
        seq -= 1;
        x %= size;
    }

    2usize.pow(seq)
}
