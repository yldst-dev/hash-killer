import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowSize = NSSize(width: 1060, height: 800)
    let windowFrame = NSRect(origin: self.frame.origin, size: windowSize)
    self.minSize = windowSize
    self.isRestorable = false
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)
    self.center()

    RegisterGeneratedPlugins(registry: flutterViewController)

    super.awakeFromNib()
  }
}
