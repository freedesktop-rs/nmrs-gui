use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

gpui_kit::assets::icon_assets!(
    Icons,
    [
        Ban,
        Bluetooth,
        BluetoothConnected,
        BluetoothOff,
        BluetoothSearching,
        Bookmark,
        Cable,
        Check,
        ChevronDown,
        ChevronLeft,
        ChevronRight,
        CircleDot,
        EthernetPort,
        ExternalLink,
        FileUp,
        FolderOpen,
        Globe,
        LoaderCircle,
        Lock,
        Minus,
        Network,
        Palette,
        Plus,
        RadioTower,
        RefreshCw,
        Search,
        Shield,
        ShieldCheck,
        ShieldOff,
        Unplug,
        Wifi,
        WifiHigh,
        WifiLow,
        WifiOff,
        WifiPen,
        WifiZero,
        X,
    ]
);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match Icons.load(path) {
            Ok(Some(data)) => Ok(Some(data)),
            _ => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all = Icons.list(path)?;
        all.extend(gpui_kit::assets::Assets.list(path).unwrap_or_default());
        Ok(all)
    }
}
