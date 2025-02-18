use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Hunt - Index and Searching
struct Arguments {
    #[argh(positional)]
    expression: String,

    #[argh(positional)]
    directories: Vec<String>
}

fn main() {
    let up: Arguments = argh::from_env();
    println!("{:?}", up);
}

