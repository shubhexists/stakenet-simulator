# Stakent Simulator

This repo contains: 
- `stakenet-simulator-db` - This is a common library that interacts with the database schema
- `services/cli` - It is the main backtesting binary which is used to backtest the steward protocol. It calculates the APY 
of the steward protocol with the new parameters provided in the CLI.
- `services/epoch-rewards-tracker` - It is a seperate binary target that is used to inject data into the database 

## Table of Contents
- [Stakenet Cli](https://github.com/exo-tech-xyz/stakenet-simulator?tab=readme-ov-file#stakenet-cli)
- [Epoch Rewards Tracker](https://github.com/exo-tech-xyz/stakenet-simulator?tab=readme-ov-file#epoch-rewards-tracker)

## stakenet-cli
Runs a backtesting simulation with configurable steward parameters.

```bash
steward-backtest-cli backtest [OPTIONS]
```

Set `env` variables - 
- RPC_URL
- DB_CONNECTION_URL
- VALIDATOR_HISTORY_PROGRAM_ID (`HistoryJTGbKQD2mRgLZ3XhqHnN811Qpez8X9kCcGHoa`)
- EPOCH_CHECK_CYCLE_SEC

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

## epoch-rewards-tracker
### Configuration
The application uses environment variables for configuration:

### Required Environment Variables
| Variable | Description | Default |
|----------|-------------|---------|
| `RPC_URL` | Solana RPC endpoint URL | Required |
| `DB_CONNECTION_URL` | PostgreSQL connection string | `postgresql://postgres:postgres@127.0.0.1:54322/postgres` |
| `VALIDATOR_HISTORY_PROGRAM_ID` | Validator history program ID | Validator history program default |
| `EPOCH_CHECK_CYCLE_SEC` | Epoch check cycle in seconds | `60` |

### Command Line Interface

The application provides several subcommands for different data collection tasks:

```bash
epoch-rewards-tracker [OPTIONS] <COMMAND>
```

### Global Options

- `--rpc-url, -r <URL>`: Solana RPC endpoint URL
- `--db-connection-url <URL>`: PostgreSQL database connection string
- `--validator-history-program-id <ID>`: Validator history program ID
- `--epoch-check-cycle-sec <SECONDS>`: Epoch check cycle duration

### Available Commands
#### 1. Fetch Validator History
Collects and stores historical validator performance data.
```bash
epoch-rewards-tracker fetch-validator-history
```
**Purpose**: Gathers comprehensive validator metrics including performance scores, commission rates, and historical voting records.

#### 2. Fetch Cluster History
Collects cluster-wide metrics and health data.
```bash
epoch-rewards-tracker fetch-cluster-history
```
**Purpose**: Tracks overall network health, epoch transitions, and cluster-wide performance metrics.

#### 3. Get Stake Accounts
Analyzes stake account distribution across validators.
```bash
epoch-rewards-tracker get-stake-accounts
```
**Purpose**: Collects information about stake accounts, delegation patterns, and stake distribution across the validator set.

#### 4. Get Inflation Rewards
Calculates and tracks inflation rewards for validators.
```bash
epoch-rewards-tracker get-inflation-rewards
```
**Purpose**: Computes inflation rewards based on validator performance and stake amounts.

#### 5. Get Priority Fee Data for Epoch
Analyzes priority fee data for a specific epoch.
```bash
epoch-rewards-tracker get-priority-fee-data-for-epoch --epoch <EPOCH_NUMBER>
```
**Purpose**: Collects and analyzes transaction priority fees for the specified epoch, useful for fee market analysis.

#### 6. Fetch Active Stake
Processes active stake data from the database.
```bash
epoch-rewards-tracker fetch-active-stake
```
**Purpose**: Analyzes currently active stake positions. This command operates on existing database data and doesn't require RPC calls.

#### 7. Fetch Inactive Stake
Processes inactive stake data from the database.
```bash
epoch-rewards-tracker fetch-inactive-stake
```
**Purpose**: Analyzes inactive or deactivating stake positions. This command operates on existing database data.
