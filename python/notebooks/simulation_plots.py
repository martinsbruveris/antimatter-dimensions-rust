from __future__ import annotations

import marimo

__generated_with = "0.23.9"
app = marimo.App(width="medium")


@app.cell
def _():
    import time

    import marimo as mo
    import numpy as np
    import matplotlib.pyplot as plt

    import antimatter_dimensions as ad

    return ad, mo, np, plt, time


@app.cell
def _(mo):
    mo.md("""
    # Antimatter Dimensions: Simulation to Big Crunch
    """)
    return


@app.cell
def _(ad, np, plt):
    def plot_trace(result: ad.SimulationResult):
        dim_colors = plt.cm.Greys(np.linspace(1.0, 0.2, 8))

        trace = result.trace
        time_s = trace.time_ms / 1000.0
        antimatter_log10 = trace.antimatter.e
        dim_amounts_log10 = trace.dimensions.amount.e
        dim_bought = trace.dimensions.bought
        dim_boosts = trace.dim_boosts
        galaxies = trace.galaxies
        tickspeed_bought = trace.tickspeed.bought
        tickspeed_effect_log10 = trace.tickspeed.tickspeed_effect.e

        xlim = (0, time_s[-1] * 1.03)

        plt.figure(figsize=(12, 3))
        plt.plot(time_s, antimatter_log10, color="k")
        plt.xlim(xlim)
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
        plt.xlim(xlim)
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
        plt.xlim(xlim)
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
        plt.xlim(xlim)
        plt.ylabel("Count")
        plt.xlabel("Game Time (s)")
        plt.title("Dimension Boosts & Galaxies")
        plt.legend(loc="upper left")
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.show()

        plt.figure(figsize=(12, 3))
        plt.subplot(1, 2, 1)
        plt.plot(time_s, tickspeed_effect_log10, color="k")
        plt.xlim(xlim)
        plt.ylabel("log₁₀(effect)")
        plt.xlabel("Game Time (s)")
        plt.title("Tickspeed Effect")
        plt.grid(True, alpha=0.3)
        plt.subplot(1, 2, 2)
        plt.plot(time_s, tickspeed_bought, color="k")
        plt.xlim(xlim)
        plt.ylabel("Bought")
        plt.xlabel("Game Time (s)")
        plt.title("Tickspeed Upgrades Bought")
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.show()

        plt.figure(figsize=(12, 3))
        plt.subplot(1, 2, 1)
        plt.plot(time_s, trace.sacrificed.e, color="k")
        plt.xlim(xlim)
        plt.ylabel("log₁₀(sacrificed)")
        plt.xlabel("Game Time (s)")
        plt.title("Total Sacrificed")
        plt.grid(True, alpha=0.3)
        plt.subplot(1, 2, 2)
        plt.plot(time_s, trace.sacrifice_boost.e, color="k")
        plt.xlim(xlim)
        plt.ylabel("log₁₀(boost)")
        plt.xlabel("Game Time (s)")
        plt.title("Sacrifice Boost")
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.show()

    return (plot_trace,)


@app.cell
def _(ad, time):
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
def _(plot_trace, result):
    plot_trace(result)
    return


@app.cell
def _(mo):
    mo.md("""
    ## Sacrifice Threshold Sweep
    """)
    return


@app.cell
def _(ad, np, plt):
    thresholds = np.arange(1.5, 100.5, 0.5)
    times = np.empty_like(thresholds)
    MAX_TIME_S = 36_000.0

    for _idx, _thresh in enumerate(thresholds):
        _strategy = ad.StrategyConfig(sacrifice_threshold=_thresh)
        _config = ad.SimulationConfig(
            _strategy,
            snapshot_count=0,
            stop_score=ad.BIG_CRUNCH_THRESHOLD,
            stop_max_game_time_s=MAX_TIME_S,
        )
        _result = ad.simulate(_config)
        times[_idx] = _result.total_time_s

    mask = times < MAX_TIME_S
    thresholds = thresholds[mask]
    times = times[mask]

    plt.figure(figsize=(12, 4))
    plt.plot(thresholds, times, color="k", linewidth=0.8)
    plt.xlabel("Sacrifice Threshold (min gain ratio)")
    plt.ylabel("Time to Big Crunch (s)")
    plt.title("Sacrifice Threshold vs Time to Big Crunch")
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.show()
    return


@app.cell
def _(ad, np, plt):
    def _func():
        thresholds = np.arange(4, 8, 0.1)
        times = np.empty_like(thresholds)
        MAX_TIME_S = 36_000.0

        for _idx, _thresh in enumerate(thresholds):
            _strategy = ad.StrategyConfig(sacrifice_threshold=_thresh)
            _config = ad.SimulationConfig(
                _strategy,
                snapshot_count=0,
                stop_score=ad.BIG_CRUNCH_THRESHOLD,
                stop_max_game_time_s=MAX_TIME_S,
            )
            _result = ad.simulate(_config)
            times[_idx] = _result.total_time_s

        mask = times < MAX_TIME_S
        thresholds = thresholds[mask]
        times = times[mask]

        plt.figure(figsize=(12, 4))
        plt.plot(thresholds, times, ".k")
        plt.xlabel("Sacrifice Threshold (min gain ratio)")
        plt.ylabel("Time to Big Crunch (s)")
        plt.title("Sacrifice Threshold vs Time to Big Crunch")
        plt.grid(True, alpha=0.3)
        plt.tight_layout()
        plt.show()

    _func()
    return


@app.cell
def _(ad, plot_trace):
    _strategy = ad.StrategyConfig(sacrifice_threshold=6.0)
    _config = ad.SimulationConfig(
        _strategy,
        snapshot_count=5_000,
        stop_score=ad.BIG_CRUNCH_THRESHOLD,
        stop_max_game_time_s=4000,
    )
    _result = ad.simulate(_config)

    # print(
    #     f"Simulation complete: {_result.total_ticks:,} ticks, "
    #     f"{_result.total_time_s:.1f}s game time, "
    #     f"{len(_result.trace):,} snapshots"
    # )
    # print(
    #     f"Final: {_result.final_state.dim_boosts} boosts, "
    #     f"{_result.final_state.galaxies} galaxies, "
    #     f"antimatter ~1e{_result.final_state.antimatter.e:.0f}"
    # )

    plot_trace(_result)
    return


@app.cell
def _():
    return


if __name__ == "__main__":
    app.run()
