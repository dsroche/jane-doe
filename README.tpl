# {{crate}}

{{readme}}

## Name sources

*   U.S. first names: <https://www.ssa.gov/OACT/babynames/limits.html>
*   U.S. last names: <https://www.census.gov/topics/population/genealogy/data/2010_surnames.html>

Specifically, to recreate all files under `src/assets`, run:

```bash
cargo build --release
mkdir rawdata
cd rawdata
wget https://www.ssa.gov/OACT/babynames/names.zip -O us-given.zip
unzip us-given.zip
../target/release/combine-counts -c 3 yob{1920..2019}.txt -o ../src/assets/us-given.csv
wget https://www2.census.gov/topics/genealogy/2010surnames/names.zip -O us-surnames.zip
unzip us-surnames.zip
cut -d, -f1,3 Names_2010Census.csv | tail -n+2 | grep -v '^ALL OTHER NAMES,' >../src/assets/us-surnames.csv
```
