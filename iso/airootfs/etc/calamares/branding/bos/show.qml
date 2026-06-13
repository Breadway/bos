/* BOS installer slideshow */
import QtQuick 2.15
import io.calamares.ui 1.0

Presentation {
    id: presentation

    Slide {
        anchors.fill: parent

        Rectangle {
            anchors.fill: parent
            color: "#2e3440"

            Column {
                anchors.centerIn: parent
                spacing: 20

                Text {
                    text: "Bread Operating System"
                    color: "#eceff4"
                    font.pointSize: 28
                    font.bold: true
                    anchors.horizontalCenter: parent.horizontalCenter
                }

                Text {
                    text: "Installing your system…"
                    color: "#88c0d0"
                    font.pointSize: 16
                    anchors.horizontalCenter: parent.horizontalCenter
                }

                Text {
                    text: "Hyprland · bread · bakery · snapshots"
                    color: "#616e88"
                    font.pointSize: 12
                    anchors.horizontalCenter: parent.horizontalCenter
                }
            }
        }
    }
}
