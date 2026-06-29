#[derive(Clone, Copy, Debug)]
pub struct DynamicClusterNode {
    pub id: usize,
    pub weight: f64,
    pub atp: f64,
    pub activity: f64,
    pub prediction_error: f64,
    pub moving_average_error: f64,
    pub learning_rate: f64,
}
