import 'package:fimber/fimber.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../domain/model/attribute/data_attribute.dart';
import '../../navigation/wallet_routes.dart';
import '../../util/cast_util.dart';
import '../../util/extension/build_context_extension.dart';
import '../common/screen/placeholder_screen.dart';
import '../common/sheet/confirm_action_sheet.dart';
import '../common/widget/button/animated_visibility_back_button.dart';
import '../common/widget/button/icon/close_icon_button.dart';
import '../common/widget/centered_loading_indicator.dart';
import '../common/widget/fake_paging_animated_switcher.dart';
import '../common/widget/wallet_app_bar.dart';
import '../organization/approve/organization_approve_page.dart';
import '../organization/detail/organization_detail_screen.dart';
import 'argument/sign_screen_argument.dart';
import 'bloc/sign_bloc.dart';
import 'page/check_agreement_page.dart';
import 'page/confirm_agreement_page.dart';
import 'page/sign_confirm_pin_page.dart';
import 'page/sign_generic_error_page.dart';
import 'page/sign_stopped_page.dart';
import 'page/sign_success_page.dart';

class SignScreen extends StatelessWidget {
  static SignScreenArgument getArgument(RouteSettings settings) {
    final args = settings.arguments;
    try {
      return tryCast<SignScreenArgument>(args) ?? SignScreenArgument.fromMap(args as Map<String, dynamic>);
    } catch (exception, stacktrace) {
      Fimber.e('Failed to decode $args', ex: exception, stacktrace: stacktrace);
      throw UnsupportedError('Make sure to pass in [SignScreenArgument] when opening the SignScreen');
    }
  }

  const SignScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final progress = context.watch<SignBloc>().state.stepperProgress;
    return Scaffold(
      appBar: WalletAppBar(
        leading: _buildBackButton(context),
        title: Text(context.l10n.signScreenTitle),
        actions: [CloseIconButton(onPressed: () => _stopSigning(context))],
        progress: progress,
      ),
      body: PopScope(
        canPop: false,
        onPopInvoked: (didPop) {
          if (didPop) {
            return;
          }
          final bloc = context.read<SignBloc>();
          if (bloc.state.canGoBack) {
            bloc.add(const SignBackPressed());
          } else {
            _stopSigning(context);
          }
        },
        child: SafeArea(
          child: _buildPage(),
        ),
      ),
    );
  }

  Widget _buildBackButton(BuildContext context) {
    return BlocBuilder<SignBloc, SignState>(
      builder: (context, state) {
        return AnimatedVisibilityBackButton(
          visible: state.canGoBack,
          onPressed: () => context.read<SignBloc>().add(const SignBackPressed()),
        );
      },
    );
  }

  Widget _buildPage() {
    return BlocBuilder<SignBloc, SignState>(
      builder: (context, state) {
        Widget result = switch (state) {
          SignInitial() => _buildLoading(),
          SignLoadInProgress() => _buildLoading(),
          SignCheckOrganization() => _buildCheckOrganization(context, state),
          SignCheckAgreement() => _buildCheckAgreement(context, state),
          SignConfirmAgreement() => _buildConfirmAgreement(context, state),
          SignConfirmPin() => _buildConfirmPin(context, state),
          SignError() => _buildError(context),
          SignStopped() => _buildStopped(context, state),
          SignSuccess() => _buildSuccess(context, state),
        };
        final skipAnim = !state.didGoBack && state is SignCheckOrganization;
        return FakePagingAnimatedSwitcher(
          animateBackwards: state.didGoBack,
          animate: !skipAnim,
          child: result,
        );
      },
    );
  }

  Widget _buildLoading() => const CenteredLoadingIndicator();

  Widget _buildError(BuildContext context) {
    return SignGenericErrorPage(
      onClosePressed: () => Navigator.pop(context),
    );
  }

  Widget _buildCheckOrganization(BuildContext context, SignCheckOrganization state) {
    return OrganizationApprovePage(
      onDeclinePressed: () => _stopSigning(context),
      onAcceptPressed: () => context.read<SignBloc>().add(const SignOrganizationApproved()),
      organization: state.organization,
      originUrl: 'http://sign.origin.org',
      purpose: ApprovalPurpose.sign,
      onShowDetailsPressed: () {
        OrganizationDetailScreen.showPreloaded(
          context,
          state.organization,
          false,
        );
      },
    );
  }

  Widget _buildCheckAgreement(BuildContext context, SignCheckAgreement state) {
    return CheckAgreementPage(
      onDecline: () => _stopSigning(context),
      onAccept: () => context.read<SignBloc>().add(const SignAgreementChecked()),
      organization: state.organization,
      trustProvider: state.trustProvider,
      document: state.document,
    );
  }

  Widget _buildConfirmAgreement(BuildContext context, SignConfirmAgreement state) {
    if (state.requestedAttributes.any((attribute) => attribute is! DataAttribute)) {
      throw UnimplementedError('Not supported, mocks are solely based on data in PID atm.');
    }
    return ConfirmAgreementPage(
      requestedAttributes: state.requestedAttributes.whereType<DataAttribute>().toList(),
      policy: state.policy,
      trustProvider: state.trustProvider,
      onDeclinePressed: () => _stopSigning(context),
      onAcceptPressed: () => context.read<SignBloc>().add(const SignAgreementApproved()),
    );
  }

  Widget _buildConfirmPin(BuildContext context, SignConfirmPin state) {
    return SignConfirmPinPage(
      onPinValidated: (_) => context.read<SignBloc>().add(const SignPinConfirmed()),
    );
  }

  void _stopSigning(BuildContext context) async {
    final bloc = context.read<SignBloc>();
    if (bloc.state.showStopConfirmation) {
      final stopped = await ConfirmActionSheet.show(
        context,
        title: context.l10n.signScreenCancelSheetTitle,
        description: context.l10n.signScreenCancelSheetDescription,
        cancelButtonText: context.l10n.signScreenCancelSheetNegativeCta,
        confirmButtonText: context.l10n.signScreenCancelSheetPositiveCta,
        confirmButtonColor: context.colorScheme.error,
      );
      if (stopped) bloc.add(const SignStopRequested());
    } else {
      Navigator.pop(context);
    }
  }

  Widget _buildStopped(BuildContext context, SignStopped state) {
    return SignStoppedPage(
      onClosePressed: () => Navigator.pop(context),
      onGiveFeedbackPressed: () => PlaceholderScreen.show(context),
    );
  }

  Widget _buildSuccess(BuildContext context, SignSuccess state) {
    return SignSuccessPage(
      organizationName: state.organization.displayName,
      onClosePressed: () => Navigator.pop(context),
      onHistoryPressed: () => Navigator.restorablePushNamed(
        context,
        WalletRoutes.walletHistoryRoute,
      ),
    );
  }
}
