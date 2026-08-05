#!/usr/bin/env python3
import math

def leo_tid(alt, inc, days, shield=3.0):
    base = 0.5
    alt_f = math.exp((alt - 500) / 500)
    inc_f = 1.0 + abs(inc - 28.5) / 90.0
    sh_f = math.exp(-shield / 5.0)
    daily = base * alt_f * inc_f * sh_f
    total = daily * days
    return {"daily": round(daily,3), "total": round(total,1), "margin": round(120-total,1), "pass": total <= 120}

print("=" * 40)
print("QRAP Radiation Calculator")
print("=" * 40)
for name, alt, inc, days, shield in [
    ("ISS 1yr", 400, 51.6, 365, 3.0),
    ("Starlink 5yr", 550, 53, 365*5, 5.0),
]:
    r = leo_tid(alt, inc, days, shield)
    print(f"
{name}: {r["total"]} krad | Margin: {r["margin"]} | {'PASS' if r['pass'] else 'FAIL'}")
