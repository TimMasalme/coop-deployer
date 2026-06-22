use coop_domain::{errors::DeployError, models::Campaign};

use crate::ports::Ports;

pub async fn list_campaigns(ports: &Ports) -> Result<Vec<Campaign>, DeployError> {
    ports.db.list_campaigns().await.map_err(|e| DeployError::new(e.to_string()))
}

pub async fn create_campaign(ports: &Ports, name: String, map_ids: Vec<i32>) -> Result<Campaign, DeployError> {
    let campaigns = ports.db.list_campaigns().await.map_err(|e| DeployError::new(e.to_string()))?;
    let next_id = campaigns.iter().map(|c| c.id).max().unwrap_or(0) + 1;
    let campaign = Campaign { id: next_id, name, map_ids };
    ports.db.upsert_campaign(campaign.clone()).await.map_err(|e| DeployError::new(e.to_string()))?;
    Ok(campaign)
}

pub async fn update_campaign(ports: &Ports, id: i32, name: String, map_ids: Vec<i32>) -> Result<Campaign, DeployError> {
    let campaign = Campaign { id, name, map_ids };
    ports.db.upsert_campaign(campaign.clone()).await.map_err(|e| DeployError::new(e.to_string()))?;
    Ok(campaign)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::fake_ports;

    #[tokio::test]
    async fn create_campaign_assigns_id() {
        let ports = fake_ports();
        let campaign = create_campaign(&ports, "UEF Campaign".into(), vec![1, 3, 4]).await.unwrap();
        assert_eq!(campaign.id, 1);
        assert_eq!(campaign.map_ids, vec![1, 3, 4]);
    }

    #[tokio::test]
    async fn list_campaigns_returns_all() {
        let ports = fake_ports();
        create_campaign(&ports, "UEF".into(), vec![1, 2]).await.unwrap();
        create_campaign(&ports, "Aeon".into(), vec![8, 9]).await.unwrap();
        let campaigns = list_campaigns(&ports).await.unwrap();
        assert_eq!(campaigns.len(), 2);
    }

    #[tokio::test]
    async fn update_campaign_changes_map_ids() {
        let ports = fake_ports();
        create_campaign(&ports, "UEF".into(), vec![1, 2]).await.unwrap();
        update_campaign(&ports, 1, "UEF".into(), vec![1, 2, 3]).await.unwrap();
        let campaigns = list_campaigns(&ports).await.unwrap();
        let uef = campaigns.iter().find(|c| c.id == 1).unwrap();
        assert_eq!(uef.map_ids, vec![1, 2, 3]);
    }
}
