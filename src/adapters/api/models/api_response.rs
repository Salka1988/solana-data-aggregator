use serde::Serialize;

/// A generic API response wrapper that standardizes responses for the API.
///
/// This struct is used to wrap any data returned by the API endpoints with a status indicator.
/// The status field is set to "success" by default when using the `new` constructor.
///
/// # Type Parameters
///
/// * `T`: The type of the data payload, which must implement `Serialize`.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// A string indicating the status of the response.
    pub status: String,
    /// The data payload of the response.
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Constructs a new `ApiResponse` with the provided data.
    ///
    /// The status is automatically set to "success".
    ///
    /// # Arguments
    ///
    /// * `data` - The payload data to include in the response.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde::Serialize;
    ///
    /// use solana_data_aggregator::adapters::api::models::api_response::ApiResponse;
    ///
    /// #[derive(Serialize)]
    /// struct MyData {
    ///     value: i32,
    /// }
    ///
    /// let data = MyData { value: 42 };
    /// let response = ApiResponse::new(data);
    /// assert_eq!(response.status, "success");
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            status: "success".into(),
            data,
        }
    }
}
