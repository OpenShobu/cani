fn main() -> std::io::Result<()> {
    cani::run(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())
}
