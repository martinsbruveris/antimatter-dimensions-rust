import marimo

__generated_with = "0.23.9"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import numpy as np
    import matplotlib.pyplot as plt

    return mo, np, plt


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
    config = ad.SimulationConfig(
        strategy, tick_ms=50.0, snapshot_count=5_000
    )
    t0 = time.perf_counter()
    result = ad.simulate(config)
    wall_time_ms = (time.perf_counter() - t0) * 1000

    print(
        f"Simulation complete: {result.total_ticks} ticks, "
        f"{result.total_time_s:.1f}s game time, "
        f"{len(result.trace)} snapshots"
    )
    print(
        f"Final: {result.dim_boosts} boosts, "
        f"{result.galaxies} galaxies, "
        f"antimatter ~1e{result.final_antimatter.log10():.0f}"
    )
    print(f"Wall time: {wall_time_ms:.1f} ms")
    return (result,)


@app.cell
def _(np, result):
    trace = result.trace
    time_s = np.array([s.time_ms / 1000.0 for s in trace])
    antimatter_log10 = np.array(
        [s.antimatter.log10() for s in trace]
    )
    dim_amounts_log10 = np.array(
        [s.dimension_amounts.log10() for s in trace]
    )
    dim_bought = np.array(
        [s.dimension_bought for s in trace]
    )
    dim_boosts = np.array([s.dim_boosts for s in trace])
    galaxies = np.array([s.galaxies for s in trace])
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
    galaxies,
    np,
    plt,
    time_s,
):
    fig, axes = plt.subplots(4, 1, figsize=(12, 14), sharex=True)

    # Plot 1: Antimatter (log10)
    ax = axes[0]
    ax.plot(time_s, antimatter_log10, color="tab:purple")
    ax.set_ylabel("log₁₀(antimatter)")
    ax.set_title("Antimatter")
    ax.grid(True, alpha=0.3)

    # Plot 2: Dimension amounts (log10)
    ax = axes[1]
    for i in range(8):
        col = dim_amounts_log10[:, i]
        mask = np.isfinite(col) & (col > -np.inf)
        if mask.any():
            ax.plot(
                time_s[mask], col[mask], label=f"Dim {i + 1}"
            )
    ax.set_ylabel("log₁₀(amount)")
    ax.set_title("Dimension Amounts")
    ax.legend(loc="upper left", ncol=4, fontsize=8)
    ax.grid(True, alpha=0.3)

    # Plot 3: Dimensions bought
    ax = axes[2]
    for i in range(8):
        col = dim_bought[:, i]
        if col.max() > 0:
            ax.plot(time_s, col, label=f"Dim {i + 1}")
    ax.set_ylabel("Bought")
    ax.set_title("Dimension Purchases")
    ax.legend(loc="upper left", ncol=4, fontsize=8)
    ax.grid(True, alpha=0.3)

    # Plot 4: Boosts and Galaxies
    ax = axes[3]
    ax.plot(time_s, dim_boosts, label="Dim Boosts", color="tab:orange")
    ax.plot(time_s, galaxies, label="Galaxies", color="tab:blue")
    ax.set_ylabel("Count")
    ax.set_xlabel("Game Time (s)")
    ax.set_title("Dimension Boosts & Galaxies")
    ax.legend(loc="upper left")
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    fig
    return


@app.cell
def _():
    return


if __name__ == "__main__":
    app.run()
