fn main() -> std::io::Result<()> {
    use std::io::Write;

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("num.rs");
    let file = std::fs::File::create(path)?;

    writeln!(&file, "pub type P0<T = Z> = T;")?;
    for i in 1..=64 {
        writeln!(&file, "pub type P{}<T = Z> = S<P{}<T>>;", i, i - 1)?;
    }

    let path = std::path::Path::new(&out_dir).join("aliases.rs");
    let file = std::fs::File::create(path)?;

    let letters: Vec<char> = ('A'..='Z').collect();

    for i in 2..=8 {
        write!(
            &file,
            "\
/// A box that can {0} different types of elements
pub type Vari{0}<",
            i
        )?;
        for j in 0..i {
            writeln!(&file, "{},", letters[j])?;
        }
        write!(&file, "> = Vari<tlist!(")?;
        for j in 0..i {
            writeln!(&file, "{},", letters[j])?;
        }
        writeln!(&file, ")>;")?;
        write!(
            &file,
            "\
/// A pinned box that can {0} different types of elements
pub type PinVari{0}<",
            i
        )?;
        for j in 0..i {
            writeln!(&file, "{},", letters[j])?;
        }
        write!(&file, "> = PinVari<tlist!(")?;
        for j in 0..i {
            writeln!(&file, "{},", letters[j])?;
        }
        writeln!(&file, ")>;")?;
    }

    Ok(())
}
