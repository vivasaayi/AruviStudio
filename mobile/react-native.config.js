const path = require("path");

const whisperRnRoot = path.join(__dirname, "node_modules", "whisper.rn");

module.exports = {
  dependencies: {
    "whisper.rn": {
      root: whisperRnRoot,
      platforms: {
        ios: {
          podspecPath: path.join(whisperRnRoot, "whisper-rn.podspec"),
        },
      },
    },
  },
};
