#[cfg(test)]
mod tests {
    use crate::repository::FileSystemRepository;
    use crate::db::prisma;
    use crate::db::prisma::PrismaClient;
    use crate::repository::{CreateVersionError, NewPluginVersion, NewVoltInfo};
    use crate::repository::PublishError;
    use crate::repository::Repository;
    use rocket::tokio;

    async fn create_test_user(db: &PrismaClient) -> prisma::user::Data {
        db.user()
            .create(
                /* Display Name: */ "Tests".into(),
                /* Login name: */ "tests".into(),
                /* Avatar URL: */ "https://example.com".into(),
                vec![],
            )
            .exec()
            .await
            .unwrap()
    }

    async fn create_test_plugin_with_icon(
        repo: &mut FileSystemRepository,
        user: &prisma::user::Data,
        icon: Vec<u8>,
    ) -> Result<prisma::plugin::Data, PublishError> {
        let name = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .unwrap();
        repo.publish(NewVoltInfo {
            name: name.clone(),
            display_name: "My Test plugin".into(),
            description: "Dummy plugin".into(),
            author: "tests".into(),
            publisher_id: user.id,
            icon: Some(icon),
        })
            .await
    }

    async fn create_test_plugin(
        repo: &mut FileSystemRepository,
        user: &prisma::user::Data,
    ) -> prisma::plugin::Data {
        let icon = std::fs::read("test_assets/icon.png").unwrap();
        create_test_plugin_with_icon(repo, user, icon)
            .await
            .unwrap()
    }

    async fn db() -> PrismaClient {
        prisma::new_client().await.unwrap()
    }

    #[tokio::test]
    async fn publish_plugin_with_invalid_icon() {
        dotenvy::dotenv().unwrap();
        let db = db().await;
        let user = create_test_user(&db).await;
        let mut repo = FileSystemRepository::default();
        // Icons bigger than 500X500 should be considered invalid
        // Money doesn't grow in trees!
        let invalid_icon = std::fs::read("test_assets/invalid_icon.png").unwrap();
        let res = create_test_plugin_with_icon(&mut repo, &user, invalid_icon).await;
        assert_eq!(res.unwrap_err(), PublishError::InvalidIcon);
    }

    #[tokio::test]
    async fn publish_plugin_with_valid_icon() {
        let db = db().await;
        let user = create_test_user(&db).await;
        let mut repo = FileSystemRepository::default();
        // The icon is valid, so the plugin should be published successfully
        let icon = std::fs::read("test_assets/icon.png").unwrap();
        let name = names::Generator::with_naming(names::Name::Numbered)
            .next()
            .unwrap();
        let new_plugin = repo
            .publish(NewVoltInfo {
                name: name.clone(),
                display_name: format!("Test plugin {}", name),
                description: "Dummy plugin".into(),
                author: "tests".into(),
                publisher_id: user.id,
                icon: Some(icon),
            })
            .await.unwrap();
        // Make some sanity checks before assuming the code is OK
        assert_eq!(new_plugin.name, name);
        assert_eq!(new_plugin.display_name, format!("Test plugin {}", name));
        assert_eq!(new_plugin.description, "Dummy plugin");
        assert_eq!(new_plugin.author, "tests");
        assert_eq!(new_plugin.publisher_id, user.id);
    }

    #[tokio::test]
    async fn publish_version_with_valid_semver() {
        let db = db().await;
        let user = create_test_user(&db).await;
        let mut repo = FileSystemRepository::default();
        let plugin = create_test_plugin(&mut repo, &user).await;
        repo.create_version(plugin.name, NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        }).await.unwrap();
    }

    #[tokio::test]
    async fn try_publish_version_with_invalid_semver() {
        let db = db().await;
        let user = create_test_user(&db).await;
        let mut repo = FileSystemRepository::default();
        let plugin = create_test_plugin(&mut repo, &user).await;
        assert_eq!(repo.create_version(plugin.name, NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "invalid version".into(),
            wasm_file: None,
        }).await.unwrap_err(), CreateVersionError::InvalidSemVer);
    }

    #[tokio::test]
    async fn try_republish_version() {
        let db = db().await;
        let user = create_test_user(&db).await;
        let mut repo = FileSystemRepository::default();
        let plugin = create_test_plugin(&mut repo, &user).await;
        repo.create_version(plugin.name.clone(), NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.2.0".into(),
            wasm_file: None,
        }).await.unwrap();
        assert_eq!(repo.create_version(plugin.name, NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.2.0".into(),
            wasm_file: None,
        }).await.unwrap_err(), CreateVersionError::AlreadyExists);
    }

    #[tokio::test]
    async fn try_publish_version_older_version_than_latest() {
        let db = db().await;
        let user = create_test_user(&db).await;
        let mut repo = FileSystemRepository::default();
        let plugin = create_test_plugin(&mut repo, &user).await;
        repo.create_version(plugin.name.clone(), NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.2.0".into(),
            wasm_file: None,
        }).await.unwrap();
        assert_eq!(repo.create_version(plugin.name, NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        }).await.unwrap_err(), CreateVersionError::LessThanLatestVersion);
    }

    #[tokio::test]
    async fn try_publish_version_with_invalid_plugin_name() {
        let mut repo = FileSystemRepository::default();
        assert_eq!(repo.create_version("odfbojubforg ogjub3ho5bh 5go".into(), NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        }).await.unwrap_err(), CreateVersionError::NonExistentPlugin);
    }

    #[tokio::test]
    #[ignore]
    async fn publish_version_with_themes_and_wasm() {
        todo!()
    }
}
