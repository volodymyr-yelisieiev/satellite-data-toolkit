# API Design

## Public APIs
- NASA POWER: no auth, public endpoint.
- Open-Meteo: can be added as no-auth public endpoint.

## Protected APIs
- EUMETSAT: consumer key + secret.
- NREL PVWatts: API key.
- Planet/Maxar: commercial key/token.

## Required layers
- API status layer.
- Cache layer.
- Request logs.
- Request inspector.
- Source provenance.
- Rate-limit monitor.
- Export and saved data management.
