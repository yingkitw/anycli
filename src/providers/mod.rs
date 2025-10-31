//! Cloud provider implementations

pub mod aws;
pub mod azure;
pub mod gcp;
pub mod ibmcloud;
pub mod vmware;

pub use aws::AWSProvider;
pub use azure::AzureProvider;
pub use gcp::GCPProvider;
pub use ibmcloud::IBMCloudProvider;
pub use vmware::VMwareProvider;

