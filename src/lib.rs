#[macro_use]
extern crate include_dir;

use csv::{
    self,
    ReaderBuilder,
};
use num_traits::{
    Zero,
};
use rand::{
    thread_rng,
    Rng,
    distributions::{
        uniform::{
            SampleUniform,
        },
    },
};
use bloomfilter::{
    Bloom,
};
use std::{
    ops::{
        AddAssign,
    },
    hash::{
        Hash,
    },
};

const ASSETS_DIR: include_dir::Dir = include_dir!("src/assets");

/// Trait to sample from a collection.
pub trait SampleFrom {
    type Item;

    /// Randomly samples one item according using the given RNG.
    fn sample_using(&self, rng: &mut impl Rng) -> Self::Item;

    /// Randomly samples one item using a default RNG.
    #[inline(always)]
    fn sample(&self) -> Self::Item {
        self.sample_using(&mut thread_rng())
    }
}

/// A stream of unique random samples from an underlying sampler.
///
/// The uniqueness of returned elements is guaranteed, but the iterator
/// may hang if there are insufficiently many unique values in the underlying
/// collection.
///
/// Ensuring that the number of unique values is at least 2 times `count` should
/// be sufficient.
pub struct UniqueSampler<'a, S: SampleFrom, R: Rng> {
    source: &'a S,
    seen: Bloom<S::Item>,
    remaining: usize,
    rng: &'a mut R,
}

impl<'a, S: SampleFrom, R: Rng> UniqueSampler<'a,S,R>
where S::Item: Hash,
{
    /// Create a stream of unique random samples from the underlying collection.
    pub fn new(source: &'a S, count: usize, rng: &'a mut R) -> Self {
        Self {
            source,
            seen: Bloom::new_for_fp_rate(count, 0.1),
            remaining: count,
            rng,
        }
    }
}

impl<'a, S: SampleFrom, R: Rng> Iterator for UniqueSampler<'a,S,R>
where S::Item: Hash,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        loop {
            let x = self.source.sample_using(self.rng);
            if ! self.seen.check_and_set(&x) {
                self.remaining -= 1;
                return Some(x);
            }
        }
    }
}



/// Store values along with their frequencies.
///
/// This supports random sampling proportional to the frequencies.
/// It is enforced tha the collection is never empty (and has positive
/// frequencies), so that sampling is always safe without checking.
///
/// `N` should be a numeric type such as `u64` or `f32`.
///
/// `T` should be the type of object being stored.
pub struct FreqChoice<N,T> {
    data: Vec<(N, T)>,
    total: N,
}

impl<N,T> FreqChoice<N,T>
where N: Copy + Default + PartialOrd + AddAssign + Zero,
{
    /// Creates a list of items from (item, frequency) pairs.
    ///
    /// If any of the following conditions are not met, the collection
    /// is invalid and `None` is returned:
    ///
    /// *   There must be at least one item with positive frequency.
    /// *   All items must have non-negative frequency.
    ///
    /// For best efficiency in sampling, the caller should ensure that
    /// the list of items is sorted in decreasing order of frequencies,
    /// and there are no duplicate values.
    pub fn from_items(items: impl IntoIterator<Item=(N,T)>) -> Option<Self> {
        let mut cumulative_freq = N::zero();
        let iter = items.into_iter();
        let mut out = match iter.size_hint() {
            (_, Some(upper)) => Vec::with_capacity(upper),
            _ => Vec::new(),
        };
        for (freq, value) in iter {
            if freq < N::zero() {
                return None;
            }
            cumulative_freq += freq;
            out.push((cumulative_freq, value));
        }
        if cumulative_freq <= N::zero() {
            return None;
        };
        Some(Self {
            data: out,
            total: cumulative_freq,
        })
    }
}

impl<N,T> FreqChoice<N,T>
where N: Copy + Zero + PartialOrd + SampleUniform,
      T: Clone,
{
    fn sample_at(&self, x: N) -> T {
        if x < self.data[0].0 {
            return self.data[0].1.clone();
        };
        let n = self.data.len();
        debug_assert!(n >= 2);
        let mut lb = 1;
        let mut ub = 1;
        // invariant: ub < n && x >= count[lb - 1]
        while x >= self.data[ub].0 {
            lb = ub + 1;
            ub *= 2;
            if ub >= n {
                ub = n  - 1;
                break;
            }
        }
        // now: x >= count[lb-1] && x < count[ub]
        while lb < ub {
            let middle = (lb + ub) / 2;
            if x < self.data[middle].0 {
                ub = middle;
            } else {
                lb = middle + 1;
            }
        }
        debug_assert!(lb == ub);
        self.data[lb].1.clone()
    }
}

impl<N,T> SampleFrom for FreqChoice<N,T>
where N: Copy + Zero + PartialOrd + SampleUniform,
      T: Clone,
{
    type Item = T;

    #[inline(always)]
    fn sample_using(&self, rng: &mut impl Rng) -> Self::Item {
        self.sample_at(rng.gen_range(N::zero() .. self.total))
    }
}

struct SamplerPair<A,B,F> {
    first: A,
    second: B,
    combiner: F,
}

impl<A,B,F,T> SampleFrom for SamplerPair<A,B,F>
where A: SampleFrom,
      B: SampleFrom,
      F: Fn(<A as SampleFrom>::Item, <B as SampleFrom>::Item) -> T
{
    type Item = T;

    fn sample_using(&self, rng: &mut impl Rng) -> Self::Item {
        (self.combiner)(self.first.sample_using(rng), self.second.sample_using(rng))
    }
}

#[derive(Debug)]
pub enum CsvSource {
    USGiven,
    USSurnames,
}

fn get_asset_file(src: CsvSource) -> &'static [u8] {
    let fname = match src {
        CsvSource::USGiven => "us-given.csv",
        CsvSource::USSurnames => "us-surnames.csv",
    };
    ASSETS_DIR.get_file(fname)
        .expect(&format!("missing asset file '{}'", fname))
        .contents()
}

pub fn get_source_sampler(src: CsvSource) -> impl SampleFrom<Item=String> {
    FreqChoice::from_items(
        ReaderBuilder::new()
            .has_headers(false)
            .from_reader(get_asset_file(src))
            .into_records()
            .map(|recres| {
                let line = recres.expect("mis-formatted csv asset file");
                (str::parse::<u64>(line.get(1).expect("missing count in csv asset file"))
                 .expect("invalid count in csv asset file"),
                 line.get(0).expect("missing name in csv asset file").to_string())
            })
    ).expect("non-positive frequency in csv asset file")
}

/// Get a sampler for American names (given + surname).
pub fn us_names() -> impl SampleFrom<Item=String> {
    SamplerPair {
        first: get_source_sampler(CsvSource::USGiven),
        second: get_source_sampler(CsvSource::USSurnames),
        combiner: |mut first: String, last: String| {
            first += " ";
            first += &last;
            first
        },
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
