use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct gsi_auth {
        pub token: String
    }

#[derive(Serialize, Deserialize)]
pub struct gsi_output {
        pub precision: String,
        pub precision_position: String,
        pub precision_vector: String
    }

#[derive(Serialize, Deserialize)]
pub struct gsi_data {
        pub map_round_wins: String,
        pub map: String,
        pub player_id: String,
        pub player_match_stats: String,
        pub player_state: String,
        pub player_weapons: String,
        pub provider: String,
        pub round: String,
    
        pub allgrenades: String,
        pub allplayers_id: String,
        pub allplayers_match_stats: String,
        pub allplayers_position: String,
        pub allplayers_state: String,
        pub allplayers_weapons: String,
        pub bomb: String,
        pub phase_countdowns: String,
        pub player_position: String
    }


#[derive(Serialize, Deserialize)]
#[serde(rename = "QTShock")]
pub struct gsi_cfg {
        pub uri: String,
        pub timeout: String,
        pub buffer: String,
        pub throttle: String,
        pub heartbeat: String,
        pub auth: gsi_auth,
        pub output: gsi_output,
        pub data: gsi_data
    }
