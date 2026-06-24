import React from "react";
import { ActivityIndicator, Text, View } from "react-native";
import { WebView } from "react-native-webview";
import { styles } from "../styles/appStyles";

type MobileRemoteWebViewProps = {
  webViewRef: React.RefObject<WebView | null>;
  remoteUrl: string;
  reloadKey: number;
  bootstrapScript: string;
};

export function MobileRemoteWebView({
  webViewRef,
  remoteUrl,
  reloadKey,
  bootstrapScript,
}: MobileRemoteWebViewProps) {
  return (
    <WebView
      ref={webViewRef}
      key={`${remoteUrl}-${reloadKey}`}
      source={{ uri: remoteUrl }}
      style={styles.webView}
      injectedJavaScriptBeforeContentLoaded={bootstrapScript}
      onLoadEnd={() => webViewRef.current?.injectJavaScript(bootstrapScript)}
      javaScriptEnabled
      domStorageEnabled
      allowsInlineMediaPlayback
      mediaCapturePermissionGrantType="grantIfSameHostElsePrompt"
      startInLoadingState
      renderLoading={() => (
        <View style={styles.loading}>
          <ActivityIndicator color="#7bc8ff" />
        </View>
      )}
      renderError={(_, __, description) => (
        <View style={styles.errorPanel}>
          <Text style={styles.errorTitle}>Remote UI unavailable</Text>
          <Text style={styles.errorText}>{description}</Text>
        </View>
      )}
    />
  );
}
