//! Small JSON writers. A JSON library costs far more wasm fuel than a callback allows.

pub fn push_u(out: &mut String, mut n: u64) {
    let mut buf = [0u8; 20];
    let mut index = buf.len();
    loop {
        index -= 1;
        buf[index] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for byte in &buf[index..] {
        out.push(*byte as char);
    }
}

/// A non-negative number with up to three decimals.
pub fn num(out: &mut String, value: f64) {
    let x = if value.is_finite() && value > 0.0 { value } else { 0.0 };
    let scaled = (x * 1000.0 + 0.5) as u64;
    push_u(out, scaled / 1000);
    let mut fraction = scaled % 1000;
    if fraction != 0 {
        out.push('.');
        let mut divisor = 100;
        while fraction != 0 {
            out.push((b'0' + (fraction / divisor) as u8) as char);
            fraction %= divisor;
            divisor /= 10;
        }
    }
}

pub fn rgba(out: &mut String, color: [f32; 4]) {
    out.push('[');
    for (index, channel) in color.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        num(out, f64::from(*channel));
    }
    out.push(']');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_colours_are_plain_json() {
        let mut out = String::new();
        num(&mut out, 0.5);
        out.push(' ');
        num(&mut out, 12.0);
        out.push(' ');
        rgba(&mut out, [1.0, 0.0, 0.33, 0.5]);
        assert_eq!(out, "0.5 12 [1,0,0.33,0.5]");
    }
}
