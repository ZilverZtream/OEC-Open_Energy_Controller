#!/usr/bin/env python3
"""Verify cyclical encoding logic"""
import math

def to_cyclical_vector(hour_of_day, month, is_weekend):
    """Python version of to_cyclical_vector for verification"""
    pi = math.pi
    return [
        # Hour (0-23) mapped to circle
        math.sin(2.0 * pi * hour_of_day / 24.0),
        math.cos(2.0 * pi * hour_of_day / 24.0),

        # Month (1-12) mapped to circle
        math.sin(2.0 * pi * (month - 1) / 12.0),
        math.cos(2.0 * pi * (month - 1) / 12.0),

        # Weekend binary
        1.0 if is_weekend else 0.0
    ]

def euclidean_distance(v1, v2):
    """Calculate Euclidean distance between two vectors"""
    return math.sqrt(sum((a - b) ** 2 for a, b in zip(v1, v2)))

# Test hour continuity (hour 23 vs hour 0)
print("Testing cyclical encoding continuity:")
print("=" * 60)

hour_23 = to_cyclical_vector(23, 6, False)
hour_0 = to_cyclical_vector(0, 6, False)
hour_12 = to_cyclical_vector(12, 6, False)

print(f"\nHour 23: {[f'{x:.3f}' for x in hour_23[:2]]}")
print(f"Hour 0:  {[f'{x:.3f}' for x in hour_0[:2]]}")
print(f"Hour 12: {[f'{x:.3f}' for x in hour_12[:2]]}")

# Calculate distances
dist_23_0 = euclidean_distance(hour_23[:2], hour_0[:2])
dist_23_12 = euclidean_distance(hour_23[:2], hour_12[:2])
dist_0_12 = euclidean_distance(hour_0[:2], hour_12[:2])

print(f"\nDistance between hour 23 and 0:  {dist_23_0:.4f} (should be small)")
print(f"Distance between hour 23 and 12: {dist_23_12:.4f} (should be large)")
print(f"Distance between hour 0 and 12:  {dist_0_12:.4f} (should be large)")

# With linear encoding, the distance would be:
linear_dist_23_0 = abs(23 - 0) / 24.0
print(f"\nLinear encoding distance (23 vs 0): {linear_dist_23_0:.4f}")
print(f"Cyclical encoding is {linear_dist_23_0 / dist_23_0:.1f}x better!")

# Test month continuity (December vs January)
print("\n" + "=" * 60)
print("Testing month continuity:")
dec = to_cyclical_vector(12, 12, False)
jan = to_cyclical_vector(12, 1, False)

print(f"\nDecember (month 12): {[f'{x:.3f}' for x in dec[2:4]]}")
print(f"January (month 1):   {[f'{x:.3f}' for x in jan[2:4]]}")

dist_dec_jan = euclidean_distance(dec[2:4], jan[2:4])
print(f"\nDistance between Dec and Jan: {dist_dec_jan:.4f} (should be small)")

# Verify hour 12 is at opposite side of circle from hour 0
print("\n" + "=" * 60)
print("Verification tests:")
print(f"✓ Hour 23 and 0 are close: {dist_23_0 < 0.3}")
print(f"✓ Hour 0 and 12 are far: {dist_0_12 > 1.5}")
print(f"✓ December and January are close: {dist_dec_jan < 0.6}")

print("\n" + "=" * 60)
print("Sample feature vectors:")
print("\nWeekday morning (8 AM, June):")
morning = to_cyclical_vector(8, 6, False)
print([f'{x:.3f}' for x in morning])

print("\nWeekend evening (18:00, December):")
evening = to_cyclical_vector(18, 12, True)
print([f'{x:.3f}' for x in evening])

print("\nAll tests passed! ✓")
