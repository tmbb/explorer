defmodule Explorer.RandomDistBuilder do
  @moduledoc false

  # A utility to make it easier to bind native distributions.
  # This macro takes care of generating the random seed that will
  # be given to the rust functions so that things can be made reproducible.
  defmacro defdraw(call) do
    {f, args} = Macro.decompose_call(call)

    df_func_name = :"df_#{f}"
    seed = Macro.var(:seed, __MODULE__)
    df_args = [seed | args]

    quote do
      def unquote(f)(unquote_splicing(args)) do
        # Get the maximum random seed from the calling module attribute
        unquote(seed) = :rand.uniform(@max_random_seed)

        {:ok, polars_df} =
          apply(
            Explorer.PolarsBackend.Native,
            unquote(df_func_name),
            unquote(df_args)
          )

        Explorer.PolarsBackend.Shared.create_dataframe!(polars_df)
      end
    end
  end
end

defmodule Explorer.Random do
  @moduledoc """
  Functions to efficiently generate random numbers from random number distributions.
  """
  import Explorer.RandomDistBuilder, only: [defdraw: 1]

  @max_random_seed 2**63

  @doc """
  TODO
  """
  defdraw draw_from_normal(mu, sigma, nr_of_draws)

  @doc """
  TODO
  """
  defdraw draw_from_beta(a, b, nr_of_draws)

  @doc """
  TODO
  """
  defdraw draw_from_dirichlet(alphas, nr_of_draws)

  @doc """
  TODO
  """
  defdraw draw_from_unit_sphere(nr_of_draws)

  @doc """
  TODO
  """
  defdraw draw_from_unit_ball(nr_of_draws)

  @doc """
  TODO
  """
  defdraw draw_from_unit_circle(nr_of_draws)

  @doc """
  TODO
  """
  defdraw draw_from_unit_disc(nr_pf_draws)
end
