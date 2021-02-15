use clap::{
    Arg,
    App,
};
use rand::{
    thread_rng,
};
use jane_doe::{
    UniqueSampler,
    SampleFrom,
    us_names,
};

fn process(sampler: impl SampleFrom<Item=String>, count: usize) {
    for s in UniqueSampler::new(&sampler, count, &mut thread_rng()) {
        println!("{}", s);
    }
}

fn main() {
    let args = App::new("jane-doe")
        .about("Generates random names according to population statistics.")
        .arg(Arg::with_name("count")
             .short("n")
             .long("count")
             .value_name("COUNT")
             .help("How many names to return (default 1)")
             .takes_value(true))
        .arg(Arg::with_name("locale")
             .short("l")
             .long("locale")
             .value_name("LOCALE")
             .help("Which locale to get names from (default US)")
             .takes_value(true))
        .arg(Arg::with_name("show")
             .short("s")
             .long("show-locales")
             .help("Display a listing of supported locales"))
        .get_matches();

    // XXX only locale currently supported is "US"
    if args.is_present("show") {
        println!("us");
        return;
    }
    let locale = args.value_of("locale").unwrap_or("us");
    if locale != "us" {
        panic!("unsupported locale");
    }

    let count = args.value_of("count")
        .map(|s| str::parse(s).expect("count must be a positive integer"))
        .unwrap_or(1usize);

    process(us_names(), count);
}
