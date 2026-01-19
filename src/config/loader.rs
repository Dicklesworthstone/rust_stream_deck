//! Loader for `.streamDeckProfile` ZIP archives.
#![allow(dead_code)] // Loader types are for future use

use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

use base64::Engine;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use super::db::ProfileDb;
use super::schema::{Action, Page, Profile, ProfilePackage};
use crate::error::{Result, SdError};

/// Loads a `.streamDeckProfile` archive and imports it into the database.
pub struct ProfileLoader<'db> {
    db: &'db ProfileDb,
}

#[allow(clippy::unused_self)] // Methods may use self in future iterations
impl<'db> ProfileLoader<'db> {
    /// Creates a new loader with a database reference.
    pub const fn new(db: &'db ProfileDb) -> Self {
        Self { db }
    }

    /// Loads a profile from a ZIP archive file.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<i64> {
        let file = std::fs::File::open(path.as_ref()).map_err(|e| {
            SdError::Other(format!("Failed to open profile: {e}"))
        })?;

        self.load(file)
    }

    /// Loads a profile from any readable/seekable source.
    pub fn load<R: Read + Seek>(&self, reader: R) -> Result<i64> {
        let mut archive = ZipArchive::new(reader).map_err(|e| {
            SdError::Other(format!("Invalid ZIP archive: {e}"))
        })?;

        // Parse package.json
        let package = self.read_package_json(&mut archive)?;

        // Extract profile name from the first profile we find
        let profile_name = self.find_first_profile_name(&mut archive)?;

        // Insert package
        let package_id = self.db.insert_package(
            &profile_name,
            &package.app_version,
            &package.device_model,
            package.format_version,
            package.os_type.as_deref(),
            package.os_version.as_deref(),
        )?;

        // Insert required plugins
        for plugin_uuid in &package.required_plugins {
            self.db.insert_required_plugin(package_id, plugin_uuid)?;
        }

        // Find and process all profiles
        self.process_profiles(&mut archive, package_id)?;

        Ok(package_id)
    }

    /// Reads and parses `package.json` from the archive root.
    fn read_package_json<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
    ) -> Result<ProfilePackage> {
        let mut file = archive.by_name("package.json").map_err(|e| {
            SdError::Other(format!("Missing package.json: {e}"))
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            SdError::Other(format!("Failed to read package.json: {e}"))
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            SdError::ConfigParse(format!("Invalid package.json: {e}"))
        })
    }

    /// Finds the first profile name from manifest.json files.
    fn find_first_profile_name<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
    ) -> Result<String> {
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| {
                SdError::Other(format!("Failed to read archive entry: {e}"))
            })?;

            let name = file.name().to_string();
            if name.ends_with(".sdProfile/manifest.json") {
                drop(file);
                return self.read_profile_manifest(archive, &name).map(|p| p.name);
            }
        }

        Ok("Imported Profile".to_string())
    }

    /// Reads a profile manifest from the archive.
    fn read_profile_manifest<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        path: &str,
    ) -> Result<Profile> {
        let mut file = archive.by_name(path).map_err(|e| {
            SdError::Other(format!("Failed to open {path}: {e}"))
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            SdError::Other(format!("Failed to read {path}: {e}"))
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            SdError::ConfigParse(format!("Invalid profile manifest {path}: {e}"))
        })
    }

    /// Reads a page manifest from the archive.
    fn read_page_manifest<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        path: &str,
    ) -> Result<Page> {
        let mut file = archive.by_name(path).map_err(|e| {
            SdError::Other(format!("Failed to open {path}: {e}"))
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            SdError::Other(format!("Failed to read {path}: {e}"))
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            SdError::ConfigParse(format!("Invalid page manifest {path}: {e}"))
        })
    }

    /// Processes all profiles in the archive.
    fn process_profiles<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        package_id: i64,
    ) -> Result<()> {
        // First, collect all profile paths
        let mut profile_paths = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| {
                SdError::Other(format!("Failed to read archive entry: {e}"))
            })?;

            let name = file.name().to_string();
            if name.ends_with(".sdProfile/manifest.json") {
                profile_paths.push(name);
            }
        }

        // Extract images first so we can reference them
        let image_ids = self.process_images(archive, package_id)?;

        // Process each profile
        for path in profile_paths {
            self.process_profile(archive, package_id, &path, &image_ids)?;
        }

        Ok(())
    }

    /// Processes images from the Images/ directory.
    fn process_images<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        package_id: i64,
    ) -> Result<HashMap<String, i64>> {
        let mut image_ids = HashMap::new();

        // Collect image paths first
        let mut image_paths = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| {
                SdError::Other(format!("Failed to read archive entry: {e}"))
            })?;

            let name = file.name().to_string();
            if name.contains("/Images/") && !file.is_dir() {
                image_paths.push(name);
            }
        }

        // Process each image
        for path in image_paths {
            let mut file = archive.by_name(&path).map_err(|e| {
                SdError::Other(format!("Failed to open image {path}: {e}"))
            })?;

            let mut data = Vec::new();
            file.read_to_end(&mut data).map_err(|e| {
                SdError::Other(format!("Failed to read image {path}: {e}"))
            })?;

            // Compute hash
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash = hex::encode(hasher.finalize());

            // Encode to base64
            let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);

            // Extract filename
            let filename = path.split('/').next_back().unwrap_or(&path);

            // Determine format (case-insensitive)
            let ext = Path::new(filename)
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_lowercase);
            let format = match ext.as_deref() {
                Some("jpg" | "jpeg") => "jpeg",
                _ => "png", // Default for .png and unknown
            };

            // Insert and track by filename
            let id = self.db.insert_image(
                package_id,
                filename,
                &hash,
                format,
                None, // Width - could be extracted but not essential
                None, // Height
                &base64_data,
            )?;

            image_ids.insert(filename.to_string(), id);
        }

        Ok(image_ids)
    }

    /// Processes a single profile.
    #[allow(clippy::too_many_lines)]
    fn process_profile<R: Read + Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        package_id: i64,
        manifest_path: &str,
        image_ids: &HashMap<String, i64>,
    ) -> Result<i64> {
        let profile = self.read_profile_manifest(archive, manifest_path)?;

        // Insert device
        let device_id = self.db.insert_device(
            package_id,
            &profile.device.model,
            &profile.device.uuid,
            None,
        )?;

        // Extract profile UUID from the path
        // Path format: "Profiles/<UUID>.sdProfile/manifest.json"
        let profile_uuid = manifest_path
            .strip_prefix("Profiles/")
            .and_then(|s| s.strip_suffix(".sdProfile/manifest.json"))
            .unwrap_or("unknown");

        // Insert profile
        let profile_id = self.db.insert_profile(
            package_id,
            profile_uuid,
            &profile.name,
            &profile.version,
            Some(device_id),
            Some(&profile.pages.current),
            Some(&profile.pages.default),
        )?;

        // Process each page listed in the profile
        let profile_base = manifest_path
            .strip_suffix("/manifest.json")
            .unwrap_or(manifest_path);

        for (sort_order, page_uuid) in profile.pages.pages.iter().enumerate() {
            let page_path = format!("{profile_base}/{page_uuid}/manifest.json");

            let is_default = *page_uuid == profile.pages.default;

            // Try to read page manifest
            let page_result = self.read_page_manifest(archive, &page_path);
            if let Ok(page) = page_result {
                let page_id = self.db.insert_page(
                    profile_id,
                    page_uuid,
                    None, // Name is not in manifest
                    is_default,
                    i32::try_from(sort_order).unwrap_or(0),
                )?;

                // Process actions in controllers
                self.process_page_actions(&page, page_id, image_ids)?;
            }
        }

        Ok(profile_id)
    }

    /// Processes actions on a page.
    fn process_page_actions(
        &self,
        page: &Page,
        page_id: i64,
        image_ids: &HashMap<String, i64>,
    ) -> Result<()> {
        for controller in &page.controllers {
            for (position, action) in &controller.actions {
                self.process_action(position, action, page_id, image_ids)?;
            }
        }
        Ok(())
    }

    /// Processes a single action.
    fn process_action(
        &self,
        position: &str,
        action: &Action,
        page_id: i64,
        image_ids: &HashMap<String, i64>,
    ) -> Result<()> {
        // Parse position "row,col"
        let parts: Vec<&str> = position.split(',').collect();
        if parts.len() != 2 {
            return Ok(()); // Skip invalid positions
        }

        let row: i32 = parts[0].parse().unwrap_or(0);
        let col: i32 = parts[1].parse().unwrap_or(0);

        // Serialize settings to JSON
        let settings_json = serde_json::to_string(&action.settings.inner).ok();

        let action_id = self.db.insert_action(
            page_id,
            &action.action_id,
            row,
            col,
            &action.name,
            &action.uuid,
            Some(&action.plugin.name),
            Some(&action.plugin.version),
            action.linked_title,
            i32::try_from(action.state).unwrap_or(0),
            settings_json.as_deref(),
        )?;

        // Process action states
        for (idx, state) in action.states.iter().enumerate() {
            let image_id = state
                .image
                .as_ref()
                .and_then(|img_name| image_ids.get(img_name).copied());

            self.db.insert_action_state(
                action_id,
                i32::try_from(idx).unwrap_or(0),
                state.title.as_deref(),
                state.title_alignment.as_deref(),
                state.title_color.as_deref(),
                state.show_title.unwrap_or(true),
                state.font_family.as_deref(),
                state.font_size.and_then(|s| i32::try_from(s).ok()),
                state.font_style.as_deref(),
                state.font_underline,
                state.outline_thickness.and_then(|t| i32::try_from(t).ok()),
                image_id,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Creates a minimal test ZIP archive with profile data.
    fn create_test_archive() -> Vec<u8> {
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let buf = Vec::new();
        let cursor = Cursor::new(buf);
        let mut zip = ZipWriter::new(cursor);

        let options = SimpleFileOptions::default();

        // Add package.json
        zip.start_file("package.json", options).unwrap();
        let package = r#"{
            "AppVersion": "7.1.0",
            "DeviceModel": "TEST",
            "FormatVersion": 1,
            "RequiredPlugins": []
        }"#;
        std::io::Write::write_all(&mut zip, package.as_bytes()).unwrap();

        // Add profile manifest
        zip.start_file("Profiles/test-uuid.sdProfile/manifest.json", options)
            .unwrap();
        let profile = r#"{
            "Device": { "Model": "TEST", "UUID": "device-uuid" },
            "Name": "Test Profile",
            "Pages": { "Current": "page1", "Default": "page1", "Pages": ["page1"] },
            "Version": "3.0"
        }"#;
        std::io::Write::write_all(&mut zip, profile.as_bytes()).unwrap();

        // Add page manifest
        zip.start_file("Profiles/test-uuid.sdProfile/page1/manifest.json", options)
            .unwrap();
        let page = r#"{"Controllers": [{"Actions": {}}]}"#;
        std::io::Write::write_all(&mut zip, page.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn test_load_archive() {
        let db = ProfileDb::in_memory().unwrap();
        let loader = ProfileLoader::new(&db);

        let archive_data = create_test_archive();
        let cursor = Cursor::new(archive_data);

        let package_id = loader.load(cursor).unwrap();
        assert_eq!(package_id, 1);

        let packages = db.list_packages().unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "Test Profile");
    }

    #[test]
    #[ignore = "requires sample_configs/Default Profile.streamDeckProfile to exist"]
    fn test_load_real_profile() {
        let sample_path = "sample_configs/Default Profile.streamDeckProfile";
        if !std::path::Path::new(sample_path).exists() {
            return;
        }

        let db = ProfileDb::in_memory().unwrap();
        let loader = ProfileLoader::new(&db);

        let package_id = loader.load_file(sample_path).unwrap();
        assert!(package_id > 0);

        let packages = db.list_packages().unwrap();
        assert_eq!(packages.len(), 1);
        println!("Loaded profile: {}", packages[0].name);
    }
}
