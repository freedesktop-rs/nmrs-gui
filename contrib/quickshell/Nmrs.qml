import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: root

    property string appId: "nmrs-popup"
    property var toggleCommand: ["nmrs-gui", "--toggle", "--app-id", appId, "--layout", "compact"]
    property var rightClickCommand: ["nmrs-gui", "--toggle", "--page", "vpn", "--app-id", appId, "--layout", "compact"]
    property string font: "JetBrainsMono Nerd Font"
    property int fontSize: 12
    property int padding: 10
    property bool showLabel: true
    property int maxLabel: 18

    property var status: null

    readonly property var theme: status ? status.theme : null
    readonly property color fg: theme ? theme.foreground : "#dddddd"
    readonly property color muted: theme ? theme.muted : "#888888"
    readonly property color accent: theme ? theme.primary : "#7aa2f7"
    readonly property color hover: theme ? Qt.rgba(fg.r, fg.g, fg.b, 0.09) : "#22ffffff"

    readonly property bool online: status ? status.connectivity === "online" : false
    readonly property bool portal: status ? status.connectivity === "portal" : false
    readonly property bool vpnUp: status ? status.vpn.length > 0 : false
    readonly property var primary: status ? status.primary : null
    readonly property var wifi: status ? status.wifi : null

    readonly property string icon: {
        if (!status)
            return "󰤫";
        if (status.airplane)
            return "󰀝";
        if (portal)
            return "󰗹";
        if (!primary)
            return status.radios.wifi.enabled ? "󰤭" : "󰤮";
        if (primary.kind === "ethernet")
            return "󰈀";
        const s = wifi ? wifi.strength : 0;
        if (s >= 80)
            return "󰤨";
        if (s >= 60)
            return "󰤥";
        if (s >= 40)
            return "󰤢";
        if (s >= 20)
            return "󰤟";
        return "󰤯";
    }

    readonly property string label: {
        if (!status)
            return "";
        if (status.airplane)
            return "airplane";
        if (!primary)
            return status.radios.wifi.enabled ? "offline" : "off";
        const name = primary.name;
        return name.length > maxLabel ? name.slice(0, maxLabel - 1) + "…" : name;
    }

    readonly property string tooltip: {
        if (!status)
            return "nmrs-gui is not running";
        const lines = [];
        if (primary)
            lines.push(primary.name + " · " + primary.interface);
        if (wifi)
            lines.push(wifi.security + " · " + (wifi.band ?? "") + " · " + wifi.strength + "%" + (wifi.ip4 ? " · " + wifi.ip4 : ""));
        for (var i = 0; i < status.ethernet.length; i++) {
            const e = status.ethernet[i];
            lines.push(e.interface + (e.speed_mbps ? " · " + e.speed_mbps + " Mb/s" : "") + (e.ip4 ? " · " + e.ip4 : ""));
        }
        for (var j = 0; j < status.vpn.length; j++)
            lines.push("󰦝 " + status.vpn[j].name + " · " + status.vpn[j].kind);
        lines.push(status.connectivity);
        return lines.join("\n");
    }

    implicitWidth: row.implicitWidth + padding * 2
    implicitHeight: 30

    Process {
        id: stream
        command: ["nmrs-gui", "--status", "--watch"]
        running: true

        stdout: SplitParser {
            onRead: data => {
                try {
                    root.status = JSON.parse(data);
                } catch (e) {}
            }
        }

        onExited: {
            root.status = null;
            restart.start();
        }
    }

    Timer {
        id: restart
        interval: 3000
        onTriggered: stream.running = true
    }

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: mouse.containsMouse ? root.hover : "transparent"
    }

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 6

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.icon
            color: root.portal ? root.accent : root.primary ? root.fg : root.muted
            font.family: root.font
            font.pixelSize: root.fontSize + 2
        }

        Text {
            visible: root.showLabel && root.label !== ""
            anchors.verticalCenter: parent.verticalCenter
            text: root.label
            color: root.primary ? root.fg : root.muted
            font.family: root.font
            font.pixelSize: root.fontSize
        }

        Text {
            visible: root.vpnUp
            anchors.verticalCenter: parent.verticalCenter
            text: "󰦝"
            color: root.accent
            font.family: root.font
            font.pixelSize: root.fontSize - 2
        }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton

        onClicked: event => {
            if (event.button === Qt.RightButton)
                Quickshell.execDetached(root.rightClickCommand);
            else
                Quickshell.execDetached(root.toggleCommand);
        }
    }
}
