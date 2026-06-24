import marimo

__generated_with = "0.23.9"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import numpy as np
    import matplotlib.pyplot as plt

    dim_colors = plt.cm.Greys(np.linspace(1.0, 0.2, 8))
    return dim_colors, mo, plt


@app.cell
def _(mo):
    mo.md("""
    # Antimatter Dimensions: Simulation to Big Crunch
    """)
    return


@app.cell
def _():
    import time

    import antimatter_dimensions as ad

    strategy = ad.StrategyConfig()
    config = ad.SimulationConfig(strategy, snapshot_count=5_000)

    t0 = time.perf_counter()
    result = ad.simulate(config)
    wall_time_ms = (time.perf_counter() - t0) * 1000

    print(
        f"Simulation complete: {result.total_ticks:,} ticks, "
        f"{result.total_time_s:.1f}s game time, "
        f"{len(result.trace):,} snapshots"
    )
    print(
        f"Final: {result.final_state.dim_boosts} boosts, "
        f"{result.final_state.galaxies} galaxies, "
        f"antimatter ~1e{result.final_state.antimatter.e:.0f}"
    )
    print(f"Wall time: {wall_time_ms:.1f} ms")
    return (result,)


@app.cell
def _(result):
    trace = result.trace
    time_s = trace.time_ms / 1000.0
    antimatter_log10 = trace.antimatter.e
    dim_amounts_log10 = trace.dimensions.amount.e
    dim_bought = trace.dimensions.bought
    dim_boosts = trace.dim_boosts
    galaxies = trace.galaxies
    return (
        antimatter_log10,
        dim_amounts_log10,
        dim_boosts,
        dim_bought,
        galaxies,
        time_s,
    )


@app.cell
def _(
    antimatter_log10,
    dim_amounts_log10,
    dim_boosts,
    dim_bought,
    dim_colors,
    galaxies,
    plt,
    time_s,
):
    plt.figure(figsize=(12, 3))
    plt.plot(time_s, antimatter_log10, color="k")
    plt.ylabel("log₁₀(antimatter)")
    plt.xlabel("Game Time (s)")
    plt.title("Antimatter")
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.show()

    plt.figure(figsize=(12, 3))
    for _i in range(7, -1, -1):
        plt.plot(
            time_s,
            dim_amounts_log10[:, _i],
            label=f"Dim {_i + 1}",
            color=dim_colors[_i],
        )
    plt.ylabel("log₁₀(amount)")
    plt.xlabel("Game Time (s)")
    plt.title("Dimension Amounts")
    _h, _l = plt.gca().get_legend_handles_labels()
    plt.legend(_h[::-1], _l[::-1], loc="upper left", ncol=4, fontsize=8)
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.show()

    plt.figure(figsize=(12, 3))
    for _i in range(7, -1, -1):
        plt.plot(
            time_s,
            dim_bought[:, _i],
            label=f"Dim {_i + 1}",
            color=dim_colors[_i],
        )
    plt.ylabel("Bought")
    plt.xlabel("Game Time (s)")
    plt.title("Dimension Purchases")
    _h, _l = plt.gca().get_legend_handles_labels()
    plt.legend(_h[::-1], _l[::-1], loc="upper left", ncol=4, fontsize=8)
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.show()

    plt.figure(figsize=(12, 3))
    plt.plot(time_s, dim_boosts, label="Dim Boosts", color="k")
    plt.plot(time_s, galaxies, label="Galaxies", color="k", linestyle="--")
    plt.ylabel("Count")
    plt.xlabel("Game Time (s)")
    plt.title("Dimension Boosts & Galaxies")
    plt.legend(loc="upper left")
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.show()
    return


if __name__ == "__main__":
    app.run()
