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
