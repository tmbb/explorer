use polars::prelude::*;
use polars_ops::pivot::{pivot_stable, PivotAgg};

use polars::export::{arrow, arrow::ffi};

// use rand_distr::{UnitCircle, UnitBall, UnitDisc, Dirichlet};

use rand::distributions::Distribution;

use statrs::distribution::{
    Beta, Cauchy, Chi, ChiSquared, Dirac, Erlang, Exp,
    FisherSnedecor, Gamma, Gumbel, InverseGamma, Laplace,
    LogNormal, NegativeBinomial, Normal, Pareto,
    StudentsT, Triangular, Uniform, Weibull
};

use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha12Rng;

use std::format;
use std::iter;
use std::collections::HashMap;

use crate::datatypes::ExSeriesDtype;
use crate::ex_expr_to_exprs;
use crate::{ExDataFrame, ExExpr, ExLazyFrame, ExSeries, ExplorerError};
use either::Either;

// Loads the IO functions for read/writing CSV, NDJSON, Parquet, etc.
pub mod io;

fn to_string_names(names: Vec<&str>) -> Vec<String> {
    names.into_iter().map(|s| s.to_string()).collect()
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_transpose(
    df: ExDataFrame,
    keep_names_as: Option<&str>,
    new_col_names: Option<Vec<String>>,
) -> Result<ExDataFrame, ExplorerError> {
    let column_names = new_col_names.map(Either::Right);
    let new_df = df.clone_inner().transpose(keep_names_as, column_names)?;
    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif]
pub fn df_names(df: ExDataFrame) -> Result<Vec<String>, ExplorerError> {
    let names = df
        .get_column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    Ok(names)
}

#[rustler::nif]
pub fn df_dtypes(df: ExDataFrame) -> Result<Vec<ExSeriesDtype>, ExplorerError> {
    let mut dtypes: Vec<ExSeriesDtype> = vec![];

    for dtype in df.dtypes().iter() {
        dtypes.push(ExSeriesDtype::try_from(dtype)?)
    }

    Ok(dtypes)
}

#[rustler::nif]
pub fn df_shape(df: ExDataFrame) -> Result<(usize, usize), ExplorerError> {
    Ok(df.shape())
}

#[rustler::nif]
pub fn df_n_rows(df: ExDataFrame) -> Result<usize, ExplorerError> {
    Ok(df.height())
}

#[rustler::nif]
pub fn df_width(df: ExDataFrame) -> Result<usize, ExplorerError> {
    Ok(df.width())
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_concat_columns(dfs: Vec<ExDataFrame>) -> Result<ExDataFrame, ExplorerError> {
    let mut previous_names = PlHashSet::new();

    let cols = dfs
        .iter()
        .enumerate()
        .flat_map(|(idx, ex_df)| {
            let df = ex_df.clone_inner();

            df.get_columns()
                .iter()
                .map(|col| {
                    let name = col.name();
                    if previous_names.contains(&name.clone().to_string()) {
                        let new_name = format!("{name}_{idx}");
                        previous_names.insert(new_name.clone());
                        col.clone().rename(new_name.into()).to_owned()
                    } else {
                        previous_names.insert(name.to_string());
                        col.clone().to_owned()
                    }
                })
                .collect::<Vec<Series>>()
        })
        .collect::<Vec<Series>>();

    let out_df = DataFrame::new(cols)?;

    Ok(ExDataFrame::new(out_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_drop(df: ExDataFrame, name: &str) -> Result<ExDataFrame, ExplorerError> {
    let new_df = df.drop(name)?;
    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_select_at_idx(df: ExDataFrame, idx: usize) -> Result<Option<ExSeries>, ExplorerError> {
    let result = df.select_at_idx(idx).map(|s| ExSeries::new(s.clone()));
    Ok(result)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_pull(df: ExDataFrame, name: &str) -> Result<ExSeries, ExplorerError> {
    let series = df.column(name).map(|s| ExSeries::new(s.clone()))?;
    Ok(series)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_mask(df: ExDataFrame, mask: ExSeries) -> Result<ExDataFrame, ExplorerError> {
    if let Ok(ca) = mask.bool() {
        let new_df = df.filter(ca)?;
        Ok(ExDataFrame::new(new_df))
    } else {
        Err(ExplorerError::Other("Expected a boolean mask".into()))
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_slice_by_indices(
    df: ExDataFrame,
    indices: Vec<u32>,
    groups: Vec<&str>,
) -> Result<ExDataFrame, ExplorerError> {
    let idx = UInt32Chunked::from_vec("idx".into(), indices);
    let new_df = if groups.is_empty() {
        df.take(&idx)?
    } else {
        df.group_by_stable(groups)?.apply(|df| df.take(&idx))?
    };
    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_slice_by_series(
    df: ExDataFrame,
    series: ExSeries,
    groups: Vec<&str>,
) -> Result<ExDataFrame, ExplorerError> {
    match series.strict_cast(&DataType::UInt32) {
        Ok(casted) => {
            let idx = casted.u32()?;

            let new_df = if groups.is_empty() {
                df.take(idx)?
            } else {
                df.group_by_stable(groups)?.apply(|df| df.take(idx))?
            };

            Ok(ExDataFrame::new(new_df))
        }
        Err(_) => Err(ExplorerError::Other(
            "slice/2 expects a series of positive integers".into(),
        )),
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_sample_n(
    df: ExDataFrame,
    n: u64,
    replace: bool,
    shuffle: bool,
    seed: Option<u64>,
    groups: Vec<String>,
) -> Result<ExDataFrame, ExplorerError> {
    let n_s = Series::new("n".into(), &[n]);
    let new_df = if groups.is_empty() {
        df.sample_n(&n_s, replace, shuffle, seed)?
    } else {
        df.group_by_stable(groups)?
            .apply(|df| df.sample_n(&n_s, replace, shuffle, seed))?
    };

    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_sample_frac(
    df: ExDataFrame,
    frac: f64,
    replace: bool,
    shuffle: bool,
    seed: Option<u64>,
    groups: Vec<String>,
) -> Result<ExDataFrame, ExplorerError> {
    let frac_s = Series::new("frac".into(), &[frac]);
    let new_df = if groups.is_empty() {
        df.sample_frac(&frac_s, replace, shuffle, seed)?
    } else {
        df.group_by_stable(groups)?
            .apply(|df| df.sample_frac(&frac_s, replace, shuffle, seed))?
    };

    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif]
fn df_from_arrow_stream_pointer(stream_ptr: u64) -> Result<ExDataFrame, ExplorerError> {
    let stream_ptr = stream_ptr as *mut ffi::ArrowArrayStream;
    let stream_ref = unsafe { stream_ptr.as_mut() }
        .ok_or(ExplorerError::Other("Incorrect stream pointer".into()))?;

    let mut res = unsafe { ffi::ArrowArrayStreamReader::try_new(stream_ref) }
        .map_err(arrow_to_explorer_error)?;

    let df = match unsafe { res.next() } {
        None => DataFrame::empty(),
        Some(maybe) => {
            let mut acc = array_to_dataframe(maybe)?;

            while let Some(maybe) = unsafe { res.next() } {
                let df = array_to_dataframe(maybe)?;
                acc.vstack_mut(&df)?;
            }

            acc.align_chunks();
            acc
        }
    };

    Ok(ExDataFrame::new(df))
}

fn array_to_dataframe(
    stream_chunk: PolarsResult<Box<dyn arrow::array::Array>>,
) -> Result<DataFrame, ExplorerError> {
    let dyn_array = stream_chunk.map_err(arrow_to_explorer_error)?;

    let struct_array = dyn_array
        .as_any()
        .downcast_ref::<crate::dataframe::arrow::array::StructArray>()
        .ok_or(ExplorerError::Other(
            "Unable to downcast to StructArray in ArrowArrayStreamReader chunk".into(),
        ))?
        .clone();

    DataFrame::try_from(struct_array).map_err(ExplorerError::Polars)
}

fn arrow_to_explorer_error(error: impl std::fmt::Debug) -> ExplorerError {
    ExplorerError::Other(format!("Internal Arrow error: #{error:?}"))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_sort_by(
    df: ExDataFrame,
    by_columns: Vec<String>,
    reverse: Vec<bool>,
    maintain_order: bool,
    multithreaded: bool,
    nulls_last: bool,
    groups: Vec<String>,
) -> Result<ExDataFrame, ExplorerError> {
    let sort_options = SortMultipleOptions::new()
        .with_maintain_order(maintain_order)
        .with_multithreaded(multithreaded)
        .with_nulls_last(nulls_last)
        .with_order_descending_multi(reverse);

    let new_df = if groups.is_empty() {
        df.sort(by_columns, sort_options)?
    } else {
        df.group_by_stable(groups)?
            .apply(|df| df.sort(by_columns.clone(), sort_options.clone()))?
    };

    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_sort_with(
    data: ExDataFrame,
    expressions: Vec<ExExpr>,
    directions: Vec<bool>,
    maintain_order: bool,
    multithreaded: bool,
    nulls_last: bool,
    groups: Vec<String>,
) -> Result<ExDataFrame, ExplorerError> {
    let df = data.clone_inner();
    let exprs = ex_expr_to_exprs(expressions);

    let sort_options = SortMultipleOptions::new()
        .with_maintain_order(maintain_order)
        .with_multithreaded(multithreaded)
        .with_nulls_last(nulls_last)
        .with_order_descending_multi(directions);

    let new_df = if groups.is_empty() {
        df.lazy().sort_by_exprs(exprs, sort_options).collect()?
    } else {
        df.group_by_stable(groups)?.apply(|df| {
            df.lazy()
                .sort_by_exprs(&exprs, sort_options.clone())
                .collect()
        })?
    };

    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_slice(
    df: ExDataFrame,
    offset: i64,
    length: usize,
    groups: Vec<&str>,
) -> Result<ExDataFrame, ExplorerError> {
    let new_df = if groups.is_empty() {
        df.slice(offset, length)
    } else {
        df.group_by_stable(groups)?
            .apply(|df| Ok(df.slice(offset, length)))?
    };
    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_to_dummies(df: ExDataFrame, selection: Vec<&str>) -> Result<ExDataFrame, ExplorerError> {
    let drop_first = false;
    let dummies = df
        .select(selection)
        .and_then(|df| df.to_dummies(None, drop_first))?;

    Ok(ExDataFrame::new(dummies))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_put_column(df: ExDataFrame, series: ExSeries) -> Result<ExDataFrame, ExplorerError> {
    let mut df = df.clone();
    let s = series.clone_inner();
    let new_df = df.with_column(s)?.clone();

    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_nil_count(df: ExDataFrame) -> Result<ExDataFrame, ExplorerError> {
    let new_df = df.null_count();
    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif]
pub fn df_from_series(columns: Vec<ExSeries>) -> Result<ExDataFrame, ExplorerError> {
    let columns = columns.into_iter().map(|c| c.clone_inner()).collect();

    let df = DataFrame::new(columns)?;

    Ok(ExDataFrame::new(df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_groups(df: ExDataFrame, groups: Vec<&str>) -> Result<ExDataFrame, ExplorerError> {
    let groups = df.group_by(groups)?.groups()?;

    Ok(ExDataFrame::new(groups))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_group_indices(
    df: ExDataFrame,
    groups: Vec<&str>,
) -> Result<Vec<ExSeries>, ExplorerError> {
    let series = df
        .group_by_stable(groups)?
        .groups()?
        .column("groups")?
        .list()?
        .into_iter()
        .map(|series| ExSeries::new(series.unwrap()))
        .collect();
    Ok(series)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_pivot_wider(
    df: ExDataFrame,
    id_columns: Vec<&str>,
    pivot_column: &str,
    values_column: Vec<&str>,
    names_prefix: Option<&str>,
) -> Result<ExDataFrame, ExplorerError> {
    // We need to preserve the original ID columns with a prefix,
    // so if there is any "new column name" coming from a "value column"
    // conflicting with some ID column, we can keep that ID column and
    // the new column names.
    let mut df = df.clone_inner();
    let explorer_prefix = "__explorer_column_id__";
    let temp_id_names: Vec<String> = id_columns
        .iter()
        .map(|id_name| format!("{explorer_prefix}{id_name}"))
        .collect();

    for (id_name, new_name) in id_columns.iter().zip(&temp_id_names) {
        df.rename(id_name, new_name.into())?;
    }

    let mut new_df = pivot_stable(
        &df,
        [pivot_column],
        Some(temp_id_names),
        Some(values_column),
        false,
        Some(PivotAgg::First),
        None,
    )?;

    // Instead of using the names from the pivoted DF, we go back
    // and restore the original ID column names, so we can use our
    // algo below to preserve all columns.
    let clean_names = new_df
        .get_column_names()
        .iter()
        .map(|name| name.trim_start_matches(explorer_prefix))
        .collect();

    let mut new_names = to_string_names(clean_names);
    let mut counter: HashMap<String, u16> = HashMap::new();

    for name in new_names.iter_mut() {
        let original_name = name.clone();

        if let Some(count) = counter.get(name) {
            if let Some(prefix) = names_prefix {
                *name = format!("{prefix}{name}");
            }

            if original_name == name.clone() {
                *name = format!("{name}_{count}");
            }

            counter
                .entry(name.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
        } else {
            if !id_columns.contains(&original_name.as_str()) {
                if name == "null" {
                    *name = "nil".to_string();
                }

                if let Some(prefix) = names_prefix {
                    *name = format!("{prefix}{name}");
                }
            }

            counter.insert(name.to_string(), 1);
        }
    }

    new_df.set_column_names(&new_names)?;

    Ok(ExDataFrame::new(new_df))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_lazy(df: ExDataFrame) -> Result<ExLazyFrame, ExplorerError> {
    let new_lf = df.clone_inner().lazy();
    Ok(ExLazyFrame::new(new_lf))
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_re_dtype(pattern: &str) -> Result<ExSeriesDtype, ExplorerError> {
    let s = Series::new("dummy".into(), [""])
        .into_frame()
        .lazy()
        .with_column(col("dummy").str().extract_groups(pattern)?.alias("dummy"))
        .collect()?
        .column("dummy")?
        .clone();
    let ex_dtype = ExSeriesDtype::try_from(s.dtype())?;
    Ok(ex_dtype)
}

macro_rules! draw_from_univariate_dist {
    ($seed:expr; $dist:expr; $nr_of_draws:expr) => {
        {
            let dist = $dist.unwrap();
            let mut rng = ChaCha12Rng::seed_from_u64($seed);
    
            let draws: Vec<u64> =
                iter::successors(Some(0 as u64), |n| n.checked_add(1))
                .take($nr_of_draws as usize)
                .collect();
    
            let values: Vec<_> =
                iter::repeat_with(|| dist.sample(&mut rng))
                .take($nr_of_draws as usize)
                .collect();
    
            let column_draws: Series = Series::new("draw".into(), &draws);
            let column_values: Series = Series::new("x".into(), &values);
    
            let df: PolarsResult<DataFrame> = DataFrame::new(vec![column_draws, column_values]);
    
            Ok(ExDataFrame::new(df?))
        }
    };
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_beta(seed: u64, a: f64, b: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Beta::new(a, b); nr_of_draws)
}

// #[rustler::nif(schedule = "DirtyCpu")]
// pub fn df_draw_from_skew_normal(seed: u64, loc: f64, scale: f64, shape: f64, nr_of_draws: u64)
//             -> Result<ExDataFrame, ExplorerError> {
//     draw_from_univariate_dist!(seed; SkewNormal::new(loc, scale, shape); nr_of_draws)
// }

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_cauchy(seed: u64, median: f64, scale: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Cauchy::new(median, scale); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_chi(seed: u64, freedom: u64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Chi::new(freedom); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_chi_squared(seed: u64, freedom: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; ChiSquared::new(freedom); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_dirac(seed: u64, v: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Dirac::new(v); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_erlang(seed: u64, shape: u64, rate: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Erlang::new(shape, rate); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_exp(seed: u64, rate: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Exp::new(rate); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_fisher_snedecor(seed: u64, freedom_1: f64, freedom_2: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; FisherSnedecor::new(freedom_1, freedom_2); nr_of_draws)
}

// // TODO: find why the Distribution trait is implemented twice!
// #[rustler::nif(schedule = "DirtyCpu")]
// pub fn df_draw_from_geometric(seed: u64, p: f64, nr_of_draws: u64)
//             -> Result<ExDataFrame, ExplorerError> {
//     draw_from_univariate_dist!(seed; Geometric::new(p); nr_of_draws)
// }

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_gumbel(seed: u64, location: f64, scale: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Gumbel::new(location, scale); nr_of_draws)
}

// // TODO: find why the Distribution trait is implemented twice!
// #[rustler::nif(schedule = "DirtyCpu")]
// pub fn df_draw_from_hypergeometric(seed: u64, pop_size: u64, success_states: u64,
//                                    sample_size: u64, nr_of_draws: u64)
//             -> Result<ExDataFrame, ExplorerError> {
//     draw_from_univariate_dist!(
//         seed;
//         Hypergeometric::new(pop_size, success_states, sample_size);
//         nr_of_draws
//     )
// }

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_inverse_gamma(seed: u64, shape: f64, rate: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; InverseGamma::new(shape, rate); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_laplace(seed: u64, location: f64, scale: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Laplace::new(location, scale); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_log_normal(seed: u64, mean: f64, variance: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; LogNormal::new(mean, variance); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_gamma(seed: u64, shape: f64, rate: f64, nr_of_draws: u64)
        -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Gamma::new(shape, rate); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_negative_binomial(seed: u64, r: f64, p: f64, nr_of_draws: u64)
        -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; NegativeBinomial::new(r, p); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_normal(seed: u64, mu: f64, sigma: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Normal::new(mu, sigma); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_pareto(seed: u64, scale: f64, shape: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Pareto::new(scale, shape); nr_of_draws)
}

// #[rustler::nif(schedule = "DirtyCpu")]
// pub fn df_draw_from_poisson(seed: u64, lambda: f64, nr_of_draws: u64)
//             -> Result<ExDataFrame, ExplorerError> {
//     draw_from_univariate_dist!(seed; Poisson::new(lambda); nr_of_draws)
// }

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_students_t(seed: u64, location: f64, scale: f64, freedom: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; StudentsT::new(location, scale, freedom); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_triangular(seed: u64, min: f64, max: f64, mode: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Triangular::new(min, max, mode); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_uniform(seed: u64, min: f64, max: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Uniform::new(min, max); nr_of_draws)
}

#[rustler::nif(schedule = "DirtyCpu")]
pub fn df_draw_from_weibull(seed: u64, shape: f64, scale: f64, nr_of_draws: u64)
            -> Result<ExDataFrame, ExplorerError> {
    draw_from_univariate_dist!(seed; Weibull::new(shape, scale); nr_of_draws)
}


// Multivariate distributions

// #[rustler::nif(schedule = "DirtyCpu")]
// pub fn df_draw_from_dirichlet(seed: u64, alpha: Vec<f64>, nr_of_draws: u64)
//             -> Result<ExDataFrame, ExplorerError> {
//     let k = alpha.len() as u64;
//     let dist = Dirichlet::new(&alpha[..]).unwrap();
//     let mut rng = ChaCha12Rng::seed_from_u64(seed);

//     let draws: Vec<u64> =
//         (0..nr_of_draws)
//         .into_iter()
//         .collect();

//     let variables: Vec<f64> =
//         (0..nr_of_draws)
//         .into_iter()
//         .map(|_index| dist.sample(&mut rng))
//         .flat_map(|result| result)
//         .collect();

//     let mut columns: Vec<Series> = vec![Series::new("draw".into(), &draws)];
    
//     for variable_index in 0..k {
//         let mut col: Vec<f64> = Vec::with_capacity(nr_of_draws as usize);

//         for draw in 0..nr_of_draws {
//             col.push(variables[(draw * k + variable_index) as usize])
//         }

//         let column_name = format!("x{}", variable_index + 1).into();
//         let column: Series = Series::new(column_name, &col);

//         columns.push(column);
//     }

//     let df: PolarsResult<DataFrame> = DataFrame::new(columns);

//     Ok(ExDataFrame::new(df?))
// }


