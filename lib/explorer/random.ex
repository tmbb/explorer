defmodule Explorer.Random do
  @moduledoc """
  Functions to efficiently generate random numbers from random number distributions.
  """
  import Explorer.RandomDistBuilder, only: [defdraw: 1]
  alias Explorer.DataFrame

  @max_random_seed 2**63

  @doc """
  Draw `nr_of_draws` values from a Normal distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_normal(float(), float(), integer()) :: DataFrame.t()
  defdraw draw_from_normal(mu, sigma, nr_of_draws)

  @doc """
  Draw `nr_of_draws` values from a Beta distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_beta(float(), float(), integer()) :: DataFrame.t()
  defdraw draw_from_beta(a, b, nr_of_draws)


  @doc """
  Draw `nr_of_draws` values from a SkewNormal distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_skew_normal(float(), float(), float(), integer()) :: DataFrame.t()
  defdraw draw_from_skew_normal(loc, scale, shape, nr_of_draws)

  @doc """
  Draw `nr_of_draws` values from a Cauchy distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_cauchy(float(), float(), integer()) :: DataFrame.t()
  defdraw draw_from_cauchy(median, scale, nr_of_draws)

  @doc """
  Draw `nr_of_draws` values from a Geometric distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_geometric(float(), integer()) :: DataFrame.t()
  defdraw draw_from_geometric(p, nr_of_draws)

  @doc """
  Draw `nr_of_draws` values from a Hypergeometric distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_hypergeometric(float(), float(), float(), integer()) :: DataFrame.t()
  defdraw draw_from_hypergeometric(pop_size, success_states, sample_size, nr_of_draws)

  @doc """
  Draw `nr_of_draws` values from a LogNormal distribution.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the values
  """
  @spec draw_from_log_normal(float(), float(), integer()) :: DataFrame.t()
  defdraw draw_from_log_normal(mean, variance, nr_of_draws)

  @doc """
  Draw `nr_of_draws` values from a Dirichlet distribution with parameters `alpha`.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x1`, `:x2`, ..., `:"x#\{k}"`
      (total of `k` columns, where `k = length(alpha), type: `:f64`) -
      the generated values for each draw
  """
  @spec draw_from_dirichlet(list(float()), integer()) :: DataFrame.t()
  defdraw draw_from_dirichlet(alphas, nr_of_draws)

  @doc """
  Draw `nr_of_draws` 3D points uniformly from a Unit Sphere.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the `x` coordinate for the point
    - `:y` (type: `:f64`) - the `y` coordinate for the point
    - `:z` (type: `:f64`) - the `z` coordinate for the point
  """
  @spec draw_from_unit_sphere(integer()) :: DataFrame.t()
  defdraw draw_from_unit_sphere(nr_of_draws)

  @doc """
  Draw `nr_of_draws` 3D points uniformly from a Unit Ball (the interior of the sphere).

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the `x` coordinate for the point
    - `:y` (type: `:f64`) - the `y` coordinate for the point
    - `:z` (type: `:f64`) - the `z` coordinate for the point
  """
  @spec draw_from_unit_ball(integer()) :: DataFrame.t()
  defdraw draw_from_unit_ball(nr_of_draws)

  @doc """
  Draw `nr_of_draws` 2D points uniformly from a Unit Circle.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the `x` coordinate for the point
    - `:y` (type: `:f64`) - the `y` coordinate for the point
  """
  @spec draw_from_unit_circle(integer()) :: DataFrame.t()
  defdraw draw_from_unit_circle(nr_of_draws)

  @doc """
  Draw `nr_of_draws` 2D points uniformly from a Unit Disc.

  Returns a dataframe with the following columns:
    - `:draw` (type: `:u64`) - the number of the draw
    - `:x` (type: `:f64`) - the `x` coordinate for the point
    - `:y` (type: `:f64`) - the `y` coordinate for the point
  """
  @spec draw_from_unit_disc(integer()) :: DataFrame.t()
  defdraw draw_from_unit_disc(nr_of_draws)
end
