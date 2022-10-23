use crate::db::prisma;
use crate::db::prisma::PrismaClient;
use crate::repository::FileSystemRepository;
use crate::repository::GetResourceError;
use crate::repository::PublishError;
use crate::repository::Repository;
use crate::repository::UnpublishPluginError;
use crate::repository::YankVersionError;
use crate::repository::{CreateVersionError, NewPluginVersion, NewVoltInfo};
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
    dotenvy::dotenv().unwrap();
    prisma::new_client().await.unwrap()
}

#[tokio::test]
async fn publish_plugin_with_invalid_icon() {
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
            publisher_id: user.id,
            icon: Some(icon.clone()),
        })
        .await
        .unwrap();
    // Make some sanity checks before assuming the code is OK
    assert_eq!(new_plugin.name, format!("tests.{name}"));
    assert_eq!(new_plugin.display_name, format!("Test plugin {name}"));
    assert_eq!(new_plugin.description, "Dummy plugin");
    assert_eq!(new_plugin.publisher_id, user.id);
    let repo_icon = repo.get_plugin_icon(name.clone()).await.unwrap();
    let actual_icon = std::fs::read(format!("fs-registry/icons/{name}")).unwrap();
    assert_eq!(repo_icon, icon);
    assert_eq!(actual_icon, icon);
    assert_eq!(actual_icon, repo_icon);
}

#[tokio::test]
async fn publish_version_with_valid_semver() {
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.create_version(
        plugin.name,
        NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn try_publish_version_with_invalid_semver() {
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    assert_eq!(
        repo.create_version(
            plugin.name,
            NewPluginVersion {
                preview: false,
                themes: vec![],
                version: "invalid version".into(),
                wasm_file: None,
            }
        )
        .await
        .unwrap_err(),
        CreateVersionError::InvalidSemVer
    );
}

#[tokio::test]
async fn try_republish_version() {
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.create_version(
        plugin.name.clone(),
        NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.2.0".into(),
            wasm_file: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        repo.create_version(
            plugin.name,
            NewPluginVersion {
                preview: false,
                themes: vec![],
                version: "0.2.0".into(),
                wasm_file: None,
            }
        )
        .await
        .unwrap_err(),
        CreateVersionError::AlreadyExists
    );
}

#[tokio::test]
async fn try_publish_yanked_version() {
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.create_version(
        plugin.name.clone(),
        NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        },
    )
    .await
    .unwrap();
    repo.yank_version(plugin.name.clone(), "0.1.0".into()).await.unwrap();
    assert_eq!(
        repo.create_version(
            plugin.name,
            NewPluginVersion {
                preview: false,
                themes: vec![],
                version: "0.1.0".into(),
                wasm_file: None,
            }
        )
        .await
        .unwrap_err(),
        CreateVersionError::AlreadyExists
    );
}
#[tokio::test]
async fn try_publish_version_older_version_than_latest() {
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.create_version(
        plugin.name.clone(),
        NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.2.0".into(),
            wasm_file: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        repo.create_version(
            plugin.name,
            NewPluginVersion {
                preview: false,
                themes: vec![],
                version: "0.1.0".into(),
                wasm_file: None,
            }
        )
        .await
        .unwrap_err(),
        CreateVersionError::LessThanLatestVersion
    );
}

#[tokio::test]
async fn try_publish_version_with_invalid_plugin_name() {
    let mut repo = FileSystemRepository::default();
    assert_eq!(
        repo.create_version(
            "odfbojubforg ogjub3ho5bh 5go".into(),
            NewPluginVersion {
                preview: false,
                themes: vec![],
                version: "0.1.0".into(),
                wasm_file: None,
            }
        )
        .await
        .unwrap_err(),
        CreateVersionError::NonExistentPlugin
    );
}

#[tokio::test]
async fn publish_version_with_themes_and_wasm() {
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    let expected_themes = vec![std::fs::read_to_string("test_assets/darkest.toml").unwrap()];
    let expected_wasm = std::fs::read("test_assets/lapce-rust.wasm").unwrap();
    repo.create_version(
        plugin.name.clone(),
        NewPluginVersion {
            preview: false,
            themes: expected_themes.clone(),
            version: "0.1.0".into(),
            wasm_file: Some(expected_wasm.clone()),
        },
    )
    .await
    .unwrap();
    let themes = repo
        .get_plugin_version_themes(plugin.name.clone(), "0.1.0".into())
        .await
        .unwrap();
    assert_eq!(expected_themes, themes);
    assert_eq!(
        std::fs::read_to_string(format!(
            "fs-registry/versions/{}-0.1.0/themes/0.toml",
            plugin.name
        ))
        .unwrap(),
        expected_themes[0]
    );
    let wasm = repo
        .get_plugin_version_wasm(plugin.name.clone(), "0.1.0".into())
        .await
        .unwrap();
    assert_eq!(expected_wasm, wasm);
    assert_eq!(
        std::fs::read(format!(
            "fs-registry/versions/{}-0.1.0/plugin.wasm",
            plugin.name
        ))
        .unwrap(),
        expected_wasm
    );
}
#[tokio::test]
async fn get_yanked_version(){
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.create_version(
        plugin.name.clone(),
        NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        },
    )
    .await
    .unwrap();
    repo.yank_version(plugin.name.clone(), "0.1.0".into()).await.unwrap();
    assert!(matches!(repo.get_plugin_version(plugin.name.clone(), "0.1.0".into()).await.unwrap_err(), GetResourceError::NotFound));
}

#[tokio::test]
async fn try_yanking_twice(){
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.create_version(
        plugin.name.clone(),
        NewPluginVersion {
            preview: false,
            themes: vec![],
            version: "0.1.0".into(),
            wasm_file: None,
        },
    )
    .await
    .unwrap();
    repo.yank_version(plugin.name.clone(), "0.1.0".into()).await.unwrap();
    assert_eq!(repo.yank_version(plugin.name.clone(), "0.1.0".into()).await.unwrap_err(), YankVersionError::NonExistentOrAlreadyYanked);
}

#[tokio::test]
async fn unpublish_plugin(){
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.unpublish_plugin(plugin.name.clone()).await.unwrap();
    assert!(matches!(repo.get_plugin(plugin.name.clone()).await.unwrap_err(), GetResourceError::NotFound));
}
#[tokio::test]
async fn unpublish_plugin_twice(){
    let db = db().await;
    let user = create_test_user(&db).await;
    let mut repo = FileSystemRepository::default();
    let plugin = create_test_plugin(&mut repo, &user).await;
    repo.unpublish_plugin(plugin.name.clone()).await.unwrap();
    assert!(matches!(repo.unpublish_plugin(plugin.name.clone()).await.unwrap_err(), UnpublishPluginError::NonExistent));
}
