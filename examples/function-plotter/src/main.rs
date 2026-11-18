fn main() -> Result<(), Box<dyn std::error::Error>> {
    rsille_function_plotter::run_with_args(std::env::args().skip(1).collect())
}
