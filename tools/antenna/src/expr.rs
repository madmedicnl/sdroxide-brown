//! The equation box and the length units — MMANA-GAL's `R`/`I`/`F` constants.
//!
//! MMANA lets a wire dimension be typed as an expression: the four operators
//! `+ - * /` and parentheses, with three special constants — `R` the wavelength
//! in metres, `I` one inch, `F` one foot (`R/4` = λ/4, `5*R/8`, `30*F`, `15*I`).
//! This is that little language, plus the length-unit conversions the UI offers.
//!
//! It is deliberately tiny and dependency-free. There is no variable table and
//! no functions, because MMANA has none; a name that is not `R`, `I` or `F` is an
//! error, not a silent zero.

use std::fmt;

/// The constants an expression is evaluated against, for one frequency.
#[derive(Clone, Copy, Debug)]
pub struct Consts {
    /// `R` — the free-space wavelength, in metres.
    pub lambda_m: f64,
}

impl Consts {
    pub fn at_freq(freq_hz: f64) -> Consts {
        Consts { lambda_m: crate::wavelength_m(freq_hz) }
    }

    /// `I` — one inch in metres.
    pub const INCH_M: f64 = 0.0254;
    /// `F` — one foot in metres.
    pub const FOOT_M: f64 = 0.3048;
}

/// Why an expression would not evaluate.
#[derive(Clone, Debug, PartialEq)]
pub enum ExprError {
    Empty,
    /// A character that is not part of the language.
    Unexpected(char),
    /// Input ran out in the middle of a term.
    UnexpectedEnd,
    /// A run of digits that is not a number.
    BadNumber(String),
    /// A name that is not one of `R`, `I`, `F`.
    UnknownConstant(char),
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprError::Empty => write!(f, "empty expression"),
            ExprError::Unexpected(c) => write!(f, "unexpected '{c}'"),
            ExprError::UnexpectedEnd => write!(f, "expression ends in the middle of a term"),
            ExprError::BadNumber(s) => write!(f, "'{s}' is not a number"),
            ExprError::UnknownConstant(c) => {
                write!(f, "'{c}' is not a constant (use R for lambda, I for inch, F for foot)")
            }
        }
    }
}

impl std::error::Error for ExprError {}

/// Evaluate an MMANA-style dimension expression to metres.
pub fn eval(input: &str, c: Consts) -> Result<f64, ExprError> {
    let mut p = Parser { s: input.as_bytes(), i: 0, c };
    p.ws();
    if p.i >= p.s.len() {
        return Err(ExprError::Empty);
    }
    let v = p.expr()?;
    p.ws();
    if p.i < p.s.len() {
        return Err(ExprError::Unexpected(p.s[p.i] as char));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    c: Consts,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    /// `expr := term (('+' | '-') term)*`
    fn expr(&mut self) -> Result<f64, ExprError> {
        let mut v = self.term()?;
        loop {
            self.ws();
            match self.peek() {
                Some(b'+') => {
                    self.i += 1;
                    v += self.term()?;
                }
                Some(b'-') => {
                    self.i += 1;
                    v -= self.term()?;
                }
                _ => return Ok(v),
            }
        }
    }

    /// `term := factor (('*' | '/') factor)*`
    fn term(&mut self) -> Result<f64, ExprError> {
        let mut v = self.factor()?;
        loop {
            self.ws();
            match self.peek() {
                Some(b'*') => {
                    self.i += 1;
                    v *= self.factor()?;
                }
                Some(b'/') => {
                    self.i += 1;
                    v /= self.factor()?;
                }
                _ => return Ok(v),
            }
        }
    }

    /// `factor := ('+' | '-')? primary`
    fn factor(&mut self) -> Result<f64, ExprError> {
        self.ws();
        match self.peek() {
            Some(b'+') => {
                self.i += 1;
                self.factor()
            }
            Some(b'-') => {
                self.i += 1;
                Ok(-self.factor()?)
            }
            _ => self.primary(),
        }
    }

    /// `primary := number | constant | '(' expr ')'`
    fn primary(&mut self) -> Result<f64, ExprError> {
        self.ws();
        match self.peek() {
            None => Err(ExprError::UnexpectedEnd),
            Some(b'(') => {
                self.i += 1;
                let v = self.expr()?;
                self.ws();
                match self.peek() {
                    Some(b')') => {
                        self.i += 1;
                        Ok(v)
                    }
                    Some(c) => Err(ExprError::Unexpected(c as char)),
                    None => Err(ExprError::UnexpectedEnd),
                }
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => self.number(),
            Some(c) if c.is_ascii_alphabetic() => {
                self.i += 1;
                match c.to_ascii_uppercase() {
                    b'R' => Ok(self.c.lambda_m),
                    b'I' => Ok(Consts::INCH_M),
                    b'F' => Ok(Consts::FOOT_M),
                    other => Err(ExprError::UnknownConstant(other as char)),
                }
            }
            Some(c) => Err(ExprError::Unexpected(c as char)),
        }
    }

    fn number(&mut self) -> Result<f64, ExprError> {
        let start = self.i;
        while self.i < self.s.len() {
            let c = self.s[self.i];
            // Digits, a single '.', and a sign/exponent only inside scientific
            // notation (`1.5e-3`), which is how people type small capacitances.
            if c.is_ascii_digit() || c == b'.' {
                self.i += 1;
            } else if (c == b'e' || c == b'E')
                && self.i + 1 < self.s.len()
                && matches!(self.s[self.i + 1], b'0'..=b'9' | b'+' | b'-')
            {
                self.i += 2;
                while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                    self.i += 1;
                }
                break;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
        text.parse::<f64>().map_err(|_| ExprError::BadNumber(text.to_string()))
    }
}

/// A unit a length can be shown or typed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthUnit {
    Metres,
    Feet,
    Inches,
    Wavelength,
}

impl LengthUnit {
    /// Every unit, in the order a menu would list them.
    pub const ALL: [LengthUnit; 4] =
        [LengthUnit::Metres, LengthUnit::Feet, LengthUnit::Inches, LengthUnit::Wavelength];

    pub fn suffix(self) -> &'static str {
        match self {
            LengthUnit::Metres => "m",
            LengthUnit::Feet => "ft",
            LengthUnit::Inches => "in",
            LengthUnit::Wavelength => "\u{03bb}", // λ
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LengthUnit::Metres => "metres",
            LengthUnit::Feet => "feet",
            LengthUnit::Inches => "inches",
            LengthUnit::Wavelength => "wavelengths",
        }
    }

    /// A value in this unit converted to metres (`lambda_m` is the wavelength).
    pub fn to_metres(self, value: f64, lambda_m: f64) -> f64 {
        match self {
            LengthUnit::Metres => value,
            LengthUnit::Feet => value * Consts::FOOT_M,
            LengthUnit::Inches => value * Consts::INCH_M,
            LengthUnit::Wavelength => value * lambda_m,
        }
    }

    /// Metres shown in this unit.
    pub fn from_metres(self, metres: f64, lambda_m: f64) -> f64 {
        match self {
            LengthUnit::Metres => metres,
            LengthUnit::Feet => metres / Consts::FOOT_M,
            LengthUnit::Inches => metres / Consts::INCH_M,
            LengthUnit::Wavelength => {
                if lambda_m > 0.0 {
                    metres / lambda_m
                } else {
                    f64::NAN
                }
            }
        }
    }
}

/// Parse `"5.2"`, `"5.2m"`, `"17ft"`, `"10 in"` or `"0.5λ"` into metres.
///
/// A bare number is metres. The unit is matched case-insensitively and may be
/// separated by spaces; `l` and `lambda` are accepted for a wavelength.
pub fn parse_length(input: &str, lambda_m: f64) -> Result<f64, String> {
    let t = input.trim();
    if t.is_empty() {
        return Err("empty".into());
    }
    // Split the leading numeric part from the trailing unit.
    let num_end = t
        .char_indices()
        .find(|(_, c)| {
            !(c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+' || *c == 'e' || *c == 'E')
        })
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    let (num, unit) = t.split_at(num_end);
    let value: f64 = num.trim().parse().map_err(|_| format!("'{num}' is not a number"))?;
    let unit = unit.trim().to_ascii_lowercase();
    let u = match unit.as_str() {
        "" | "m" | "metre" | "metres" | "meter" | "meters" => LengthUnit::Metres,
        "ft" | "foot" | "feet" | "'" => LengthUnit::Feet,
        "in" | "inch" | "inches" | "\"" => LengthUnit::Inches,
        "\u{03bb}" | "l" | "lambda" | "wl" => LengthUnit::Wavelength,
        other => return Err(format!("unknown unit '{other}'")),
    };
    Ok(u.to_metres(value, lambda_m))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c() -> Consts {
        // Lambda = 10 m makes every expected value round.
        Consts { lambda_m: 10.0 }
    }

    /// The examples from MMANA's own help, verbatim.
    #[test]
    fn mmana_examples_evaluate() {
        let c = c();
        assert_eq!(eval("5*2+1", c).unwrap(), 11.0);
        assert_eq!(eval("1+5*2", c).unwrap(), 11.0);
        assert_eq!(eval("(1+5)*2", c).unwrap(), 12.0);
        assert_eq!(eval("5*(2+1)", c).unwrap(), 15.0);
        assert_eq!(eval("20/(3+7)", c).unwrap(), 2.0);
    }

    /// And its constant examples: R/4, 5*R/8, 30*F, 15*I.
    #[test]
    fn the_constants_are_lambda_inch_and_foot() {
        let c = c();
        assert!((eval("R/4", c).unwrap() - 2.5).abs() < 1e-9);
        assert!((eval("5*R/8", c).unwrap() - 6.25).abs() < 1e-9);
        assert!((eval("30*F", c).unwrap() - 9.144).abs() < 1e-9);
        assert!((eval("15*I", c).unwrap() - 0.381).abs() < 1e-9);
        // Case does not matter.
        assert!((eval("r/4", c).unwrap() - 2.5).abs() < 1e-9);
    }

    /// Precedence and unary minus.
    #[test]
    fn precedence_and_signs() {
        let c = c();
        assert_eq!(eval("-R", c).unwrap(), -10.0);
        assert_eq!(eval("2*-3", c).unwrap(), -6.0);
        assert_eq!(eval("10 - 2 - 3", c).unwrap(), 5.0);
        assert_eq!(eval("2 + 3 * 4", c).unwrap(), 14.0);
        assert_eq!(eval("1.5e-3*1000", c).unwrap(), 1.5);
    }

    /// Errors are errors, not silent zeros.
    #[test]
    fn malformed_expressions_are_refused() {
        let c = c();
        assert_eq!(eval("", c), Err(ExprError::Empty));
        assert_eq!(eval("   ", c), Err(ExprError::Empty));
        assert_eq!(eval("R/", c), Err(ExprError::UnexpectedEnd));
        assert_eq!(eval("()", c), Err(ExprError::Unexpected(')')));
        assert_eq!(eval("R+", c), Err(ExprError::UnexpectedEnd));
        assert_eq!(eval("abc", c), Err(ExprError::UnknownConstant('A')));
        assert_eq!(eval("2 2", c), Err(ExprError::Unexpected('2')));
    }

    /// Unit conversion round-trips, and matches the classic rules.
    #[test]
    fn units_convert() {
        let lam = crate::wavelength_m(27_265_000.0);
        // 468/f feet is the familiar dipole length; check the unit itself.
        assert!((LengthUnit::Feet.to_metres(1.0, lam) - 0.3048).abs() < 1e-12);
        assert!((LengthUnit::Inches.to_metres(12.0, lam) - 0.3048).abs() < 1e-12);
        assert!((LengthUnit::Wavelength.to_metres(0.25, lam) - lam / 4.0).abs() < 1e-9);
        for u in LengthUnit::ALL {
            let m = u.to_metres(3.7, lam);
            assert!((u.from_metres(m, lam) - 3.7).abs() < 1e-9, "{u:?}");
        }
    }

    /// Typed lengths accept a unit or default to metres.
    #[test]
    fn typed_lengths() {
        let lam = 10.0;
        assert!((parse_length("5.2", lam).unwrap() - 5.2).abs() < 1e-9);
        assert!((parse_length("17ft", lam).unwrap() - 5.1816).abs() < 1e-9);
        assert!((parse_length("10 in", lam).unwrap() - 0.254).abs() < 1e-9);
        assert!((parse_length("0.5\u{03bb}", lam).unwrap() - 5.0).abs() < 1e-9);
        assert!(parse_length("5 furlongs", lam).is_err());
    }
}
