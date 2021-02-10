use clap::{
    Arg,
    App,
};
use csv::{
    self,
    ReaderBuilder,
    StringRecord,
};
use std::{
    io::{
        stdout,
        Write,
        Result,
    },
    collections::{
        HashMap,
    },
};

struct Combiner {
    map: HashMap<String, u64>,
}

impl Combiner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn add(&mut self, name: &str, count: u64) {
        match self.map.get_mut(name) {
            Some(old_count) => { *old_count += count; },
            None => { self.map.insert(name.to_string(), count); },
        }
    }

    fn write_to<W: Write>(self, mut out: csv::Writer<W>) -> Result<()> {
        let mut pairs: Vec<_> = self.map.into_iter().collect();
        pairs.sort_unstable_by(
            |(_, countref1), (_, countref2)|
            countref2.cmp(countref1)
        );
        for (name, count) in pairs {
            out.write_record(&[name, count.to_string()])?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = App::new("combine-counts")
        .about("Sums up tallies form multiple csv files and writes to standard out.")
        .arg(Arg::with_name("namecol")
             .short("n")
             .long("namecol")
             .value_name("INDEX")
             .help("Which column to use for the name (default 1 = first column)")
             .takes_value(true))
        .arg(Arg::with_name("countcol")
             .short("c")
             .long("countcol")
             .value_name("INDEX")
             .help("Which column to use for the count (default 2)")
             .takes_value(true))
        .arg(Arg::with_name("outfile")
             .short("o")
             .long("outfile")
             .value_name("OUTPUT")
             .help("CSV file to write combined output (default stdout)")
             .takes_value(true))
        .arg(Arg::with_name("headers")
             .short("r")
             .long("header-row")
             .help("Indicates whether the input files have a header row (default no)"))
        .arg(Arg::with_name("INPUT")
             .help("Input file(s) in csv format")
             .multiple(true)
             .index(1)
             .required(true)
             .min_values(1))
        .get_matches();

    let out = args.value_of("outfile")
        .map(|fname| csv::Writer::from_path(fname))
        .transpose()?;

    let namecol = args.value_of("namecol")
        .map(|s| str::parse(s).expect("namecol must be an integer"))
        .unwrap_or(1) - 1;

    let countcol = args.value_of("countcol")
        .map(|s| str::parse(s).expect("countcol must be an integer"))
        .unwrap_or(2) - 1;

    let hdrs = args.is_present("headers");

    assert!(namecol != countcol);

    let mut names = Combiner::new();

    for infname in args.values_of("INPUT").unwrap() {
        let mut rdr = ReaderBuilder::new().has_headers(hdrs).from_path(infname)?;
        let mut line = StringRecord::new();
        while rdr.read_record(&mut line)? {
            let name = line.get(namecol).expect(
                &format!("Missing name on line {} of file {}",
                        line.position().map(csv::Position::line).unwrap(), infname));
            let count = str::parse(line.get(countcol)
                    .expect(&format!("Missing count on line {} of file {}",
                        line.position().map(csv::Position::line).unwrap(), infname))
                ).expect(&format!("Invalid count on line {} of file {}",
                    line.position().map(csv::Position::line).unwrap(), infname));
            names.add(name, count);
        }
    }

    match out {
        Some(filew) => names.write_to(filew),
        None => names.write_to(csv::Writer::from_writer(stdout())),
    }
}
