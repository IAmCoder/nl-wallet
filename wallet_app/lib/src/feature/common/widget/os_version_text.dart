import 'dart:io';

import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/material.dart';

import '../../../../environment.dart';
import '../../../util/extension/build_context_extension.dart';

class OsVersionText extends StatelessWidget {
  final TextStyle? prefixTextStyle;
  final TextStyle? valueTextStyle;

  const OsVersionText({
    this.prefixTextStyle,
    this.valueTextStyle,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    if (Environment.isTest) return _buildOsVersionText(context, '1.0');

    if (Platform.isAndroid) {
      return FutureBuilder<AndroidDeviceInfo>(
        future: DeviceInfoPlugin().androidInfo,
        builder: (context, snapshot) {
          final androidInfo = snapshot.data;
          if (androidInfo == null) return _buildOsVersionText(context, null);
          final release = androidInfo.version.release;
          final sdkInt = androidInfo.version.sdkInt;
          return _buildOsVersionText(context, '$release (API $sdkInt)');
        },
      );
    } else if (Platform.isIOS) {
      return FutureBuilder<IosDeviceInfo>(
        future: DeviceInfoPlugin().iosInfo,
        builder: (context, snapshot) => _buildOsVersionText(context, snapshot.data?.systemVersion),
      );
    } else {
      throw UnsupportedError('Host platform is not supported');
    }
  }

  Widget _buildOsVersionText(BuildContext context, String? versionName) {
    return Row(
      children: [
        Text(
          context.l10n.generalOsVersionText,
          style: prefixTextStyle ?? context.textTheme.bodyMedium,
        ),
        const SizedBox(width: 4),
        Text(
          versionName ?? '-',
          style: valueTextStyle ?? context.textTheme.bodyMedium,
        ),
      ],
    );
  }
}
