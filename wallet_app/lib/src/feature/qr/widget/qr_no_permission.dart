import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:permission_handler/permission_handler.dart';

import '../../../util/extension/build_context_extension.dart';
import '../../common/widget/button/text_icon_button.dart';
import '../../common/widget/utility/check_permission_on_resume.dart';
import '../bloc/qr_bloc.dart';

class QrNoPermission extends StatelessWidget {
  final bool isPermanentlyDenied;

  const QrNoPermission({required this.isPermanentlyDenied, super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      alignment: Alignment.center,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(height: context.mediaQuery.padding.top),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Text(
              context.l10n.qrScreenPermissionHint,
              textAlign: TextAlign.center,
              style: context.textTheme.bodyLarge,
            ),
          ),
          const Spacer(),
          Icon(
            Icons.camera_alt_outlined,
            color: context.colorScheme.onSurface,
          ),
          const SizedBox(height: 8),
          CheckPermissionOnResume(
            permission: Permission.camera,
            onPermissionGranted: () => context.read<QrBloc>().add(const QrScanCheckPermission()),
            child: TextIconButton(
              onPressed: () {
                if (isPermanentlyDenied) {
                  openAppSettings();
                } else {
                  context.read<QrBloc>().add(const QrScanCheckPermission());
                }
              },
              child: Text(
                context.l10n.qrScanTabGrantPermissionCta,
                textAlign: TextAlign.center,
              ),
            ),
          ),
          const Spacer(),
        ],
      ),
    );
  }
}
