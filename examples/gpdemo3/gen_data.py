#!/usr/bin/env python3
"""gpdemo3 合成数据生成器（进化图论 arc · Tier3 硬过滤/采纳测试靶）。

真值：y = expit(0.8·(d1 − 15 − d2))
  d1 = t（1..30，单调升）；d2 = max(0, 3·(t − 18))（后段抬高阈值）
→ y 先升后降（非单调 in d1）：单输入 d1 门控拟合不了 → GP 必须引入 d2、长出 d2→y 新边。
纯 stdlib、确定性、无随机。产出 drivers.csv（d1,d2） + observed.csv（DAT,y）。
"""
import math
import os

STEPS = 30
HERE = os.path.dirname(os.path.abspath(__file__))

def expit(x: float) -> float:
    return 1.0 / (1.0 + math.exp(-x))

def d1_of(t: int) -> float:
    return float(t)

def d2_of(t: int) -> float:
    return max(0.0, 3.0 * (t - 18))

def truth_y(t: int) -> float:
    return expit(0.8 * (d1_of(t) - 15.0 - d2_of(t)))

def main() -> None:
    # drivers.csv：表头即列名（每行 = 一步，无 DAT 列）
    with open(os.path.join(HERE, "drivers.csv"), "w") as f:
        f.write("d1,d2\n")
        for t in range(1, STEPS + 1):
            f.write(f"{d1_of(t):.4f},{d2_of(t):.4f}\n")
    # observed.csv：首列 DAT（1 起），逐日 y（真值，无噪，密采）
    with open(os.path.join(HERE, "observed.csv"), "w") as f:
        f.write("DAT,y\n")
        for t in range(1, STEPS + 1):
            f.write(f"{t},{truth_y(t):.6f}\n")
    peak_t = max(range(1, STEPS + 1), key=truth_y)
    print(f"gpdemo3 数据已生成：{STEPS} 步；y 峰值 {truth_y(peak_t):.3f} @DAT{peak_t}（先升后降=非单调，须 d2）")

if __name__ == "__main__":
    main()
