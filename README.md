# Stakent Simulator

This repo contains: 
- `stakenet-simulator-db` - This is a common library that interacts with the database schema
- `services/cli` - It is the main backtesting binary which is used to backtest the steward protocol. It calculates the APY 
of the steward protocol with the new parameters provided in the CLI.
- `services/epoch-rewards-tracker` - It is a seperate binary target that is used to inject data into the database 

## stakenet-cli

### backtest

Runs a backtesting simulation with configurable steward parameters.

```bash
steward-backtest-cli backtest [OPTIONS]
```

## Configuration Parameters

### Commission & MEV Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `--mev-commission-range` | `u16` | Range for MEV commission scoring |
| `--mev-commission-bps-threshold` | `u16` | MEV commission threshold in basis points |
| `--commission-range` | `u16` | Range for commission scoring |
| `--commission-threshold` | `u8` | Commission threshold percentage |
| `--historical-commission-threshold` | `u8` | Historical commission threshold |

### Epoch & Scoring Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `--epoch-credits-range` | `u16` | Range for epoch credits scoring |
| `--scoring-delinquency-threshold-ratio` | `f64` | Delinquency threshold for scoring |
| `--num-epochs-between-scoring` | `u64` | Epochs between scoring cycles |
| `--compute-score-slot-range` | `u64` | Slot range for score computation |
| `--minimum-voting-epochs` | `u64` | Minimum epochs a validator must vote |

### Unstaking Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `--instant-unstake-delinquency-threshold-ratio` | `f64` | Delinquency threshold for instant unstaking |
| `--scoring-unstake-cap-bps` | `u32` | Cap for scoring-based unstaking (basis points) |
| `--instant-unstake-cap-bps` | `u32` | Cap for instant unstaking (basis points) |
| `--stake-deposit-unstake-cap-bps` | `u32` | Cap for stake deposit unstaking (basis points) |
| `--instant-unstake-epoch-progress` | `f64` | Epoch progress threshold for instant unstaking |
| `--instant-unstake-inputs-epoch-progress` | `f64` | Input epoch progress for instant unstaking |

### Priority Fee Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `--priority-fee-lookback-epochs` | `u8` | Epochs to look back for priority fee analysis |
| `--priority-fee-lookback-offset` | `u8` | Offset for priority fee lookback |
| `--priority-fee-max-commission-bps` | `u16` | Maximum commission for priority fee (basis points) |
| `--priority-fee-error-margin-bps` | `u16` | Error margin for priority fees (basis points) |
| `--priority-fee-scoring-start-epoch` | `u16` | Starting epoch for priority fee scoring |

### Delegation Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `--num-delegation-validators` | `u32` | Number of validators to delegate to |
| `--minimum-stake-lamports` | `u64` | Minimum stake amount in lamports |

### Simulation Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `--target-epoch` | `u64` | - | Target epoch for simulation |
| `--steward-cycle-rate` | `u16` | `10` | Rate of steward cycles |
