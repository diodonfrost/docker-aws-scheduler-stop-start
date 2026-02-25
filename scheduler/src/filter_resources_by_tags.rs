use anyhow::Result;
use aws_sdk_resourcegroupstagging::types::TagFilter;
use aws_sdk_resourcegroupstagging::Client;

/// Query the AWS Resource Groups Tagging API to find resources
/// matching the given type and tag filter.
///
/// Handles pagination automatically to retrieve all results.
///
/// Returns the list of ARNs of matching resources.
pub async fn get_resources(
    client: &Client,
    resource_type: &str,
    tag_key: &str,
    tag_value: &str,
) -> Result<Vec<String>> {
    let mut arns = Vec::new();

    let tag_filter = TagFilter::builder()
        .key(tag_key)
        .values(tag_value)
        .build();

    let mut pagination_token: Option<String> = None;

    loop {
        let mut request = client
            .get_resources()
            .tag_filters(tag_filter.clone())
            .resource_type_filters(resource_type);

        if let Some(ref token) = pagination_token {
            request = request.pagination_token(token);
        }

        let response = request.send().await?;

        for mapping in response.resource_tag_mapping_list() {
            if let Some(arn) = mapping.resource_arn() {
                arns.push(arn.to_string());
            }
        }

        match response.pagination_token() {
            Some(token) if !token.is_empty() => {
                pagination_token = Some(token.to_string());
            }
            _ => break,
        }
    }

    Ok(arns)
}
