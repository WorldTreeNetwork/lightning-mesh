## ADDED Requirements

### Requirement: A walk paints region coverage

Geolocated RSSI samples SHALL paint a region as unknown, thin, or covered.
A walk that heard strong client-SSID (or joined node radio) signal SHALL
mark covered. A walk that heard nothing SHALL leave unknown. Completeness
SHALL be a visible score on `/mesh`, not a hidden metric.

#### Scenario: Strong walk

- GIVEN a recorded walk whose samples are strong RSSI through a cell
- WHEN `/mesh` consumes the walk
- THEN that cell is covered

#### Scenario: Silence

- GIVEN a recorded walk whose samples are missing or below the thin
  threshold
- WHEN `/mesh` consumes the walk
- THEN those cells stay unknown

#### Scenario: Score

- GIVEN a region with a mix of covered and unknown cells
- WHEN `/mesh` is showing the sweep
- THEN completeness is on-screen
