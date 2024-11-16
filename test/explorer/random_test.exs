defmodule Explorer.RandomTest do
  use ExUnit.Case, async: true
  use ExUnitProperties

  alias Explorer.Random
  alias Explorer.Series
  alias Explorer.DataFrame

  @max_seed 2**63

  def random_seed_gen() do
    StreamData.tuple({
      StreamData.integer(0..@max_seed),
      StreamData.integer(0..@max_seed),
      StreamData.integer(0..@max_seed)
    })
  end

  def dataframe_all_equal(df1, df2) do
    Enum.zip(DataFrame.to_series(df1), DataFrame.to_series(df2))
    |> Enum.map(fn {{_name1, s1}, {_name2, s2}} -> Series.all_equal(s1, s2) end)
    |> Enum.all?()
  end

  test "draw_from_normal is reproducible - test case" do
    :rand.seed(:exro928ss, {42, 42, 42})

    df = Random.draw_from_normal(0.0, 1.0, 5)

    assert Series.to_list(df[:value]) == [
      1.5517820245173133,
      1.321939712924412,
      0.02191481628203684,
      -0.855217344052458,
      0.6105857073818864
    ]
  end

  # Tests to ensure reproducibility

  property "draw_from_normal is reproducible if the seed is set" do
    check all seed <- random_seed_gen(),
              nr_of_draws <- StreamData.integer(0..20_000),
              mu <- StreamData.float(),
              sigma <- StreamData.float(min: 0.0) do
      # Set up a seed to make things reproducible
      :rand.seed(:exro928ss, seed)
      df1 = Random.draw_from_normal(mu, sigma, nr_of_draws)

      # Draw more numbers and discard them
      _discard = Random.draw_from_normal(0.0, 1.0, :rand.uniform(1000))

      # Set up a new seed and draw more random numbers
      :rand.seed(:exro928ss, seed)
      df2 = Random.draw_from_normal(mu, sigma, nr_of_draws)

      # The values should be equal
      assert Series.to_list(df1[:value]) == Series.to_list(df2[:value])
    end
  end

  property "draw_from_beta is reproducible if the seed is set" do
    check all seed <- random_seed_gen(),
              nr_of_draws <- StreamData.integer(0..20_000),
              a <- StreamData.float(min: 0.1),
              b <- StreamData.float(min: 0.1) do
      # Set up a seed to make things reproducible
      :rand.seed(:exro928ss, seed)
      df1 = Random.draw_from_beta(a, b, nr_of_draws)

      # Draw more numbers and discard them
      _discard = Random.draw_from_beta(1.0, 1.0, :rand.uniform(1000))

      # Set up a new seed and draw more random numbers
      :rand.seed(:exro928ss, seed)
      df2 = Random.draw_from_beta(a, b, nr_of_draws)

      # The values should be equal
      assert dataframe_all_equal(df1, df2)
    end
  end

  property "draw_from_dirichlet is reproducible if the seed is set" do
    check all seed <- random_seed_gen(),
              nr_of_draws <- StreamData.integer(0..20_000),
              alphas <- StreamData.list_of(StreamData.float(min: 0.1),
                                           min_length: 2,
                                           max_length: 32) do
      # Set up a seed to make things reproducible
      :rand.seed(:exro928ss, seed)
      df1 = Random.draw_from_dirichlet(alphas, nr_of_draws)

      # Draw more numbers and discard them
      _discard = Random.draw_from_dirichlet(alphas, :rand.uniform(1000))

      # Set up a new seed and draw more random numbers
      :rand.seed(:exro928ss, seed)
      df2 = Random.draw_from_dirichlet(alphas, nr_of_draws)

      # The values should be equal
      assert dataframe_all_equal(df1, df2)
    end
  end

  property "draw_from_dirichlet generated variables x1, x2, ... xn" do
    check all nr_of_draws <- StreamData.integer(0..5),
              alphas <- StreamData.list_of(StreamData.float(min: 0.1),
                                           min_length: 2,
                                           max_length: 32) do

      df = Random.draw_from_dirichlet(alphas, nr_of_draws)

      expected_columns = ["draw"] ++ (for i <- 1..length(alphas), do: "x#{i}")

      assert DataFrame.names(df) == expected_columns
    end
  end

  # Tests that check for problems with numerical accuracy

  property "draw_from_normal may generate repeated numbers due floating point errors" do
    assert_raise ExUnit.AssertionError, fn ->
      # Statistically, this code will fail because when StreamData starts generating
      # very large values for mu or sigma, floating point inaccuracies will cuase
      # repeated numbers to be generated
      check all seed <- random_seed_gen(),
                # nr_of_draws should be small, bug big enough so that
                # it's extremely unlikely that values will be repeated
                nr_of_draws <- StreamData.integer(5..10),
                # No caps on the mean; floating point errors will cause
                # problems with floating point addition
                mu <- StreamData.float(),
                # sigma can't be zero because otherwise there will be no randomness
                sigma <- StreamData.float(min: 0.1) do

        :rand.seed(:exro928ss, seed)

        df = Random.draw_from_normal(mu, sigma, nr_of_draws)
        values = Series.to_list(df[:value])

        # The values should be all different
        assert values == Enum.uniq(values)
      end
    end
  end
end
