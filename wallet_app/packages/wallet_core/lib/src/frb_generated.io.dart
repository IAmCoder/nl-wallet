// This file is automatically generated, so please do not edit it.
// @generated by `flutter_rust_bridge`@ 2.7.0.

// ignore_for_file: unused_import, unused_element, unnecessary_import, duplicate_ignore, invalid_use_of_internal_member, annotate_overrides, non_constant_identifier_names, curly_braces_in_flow_control_structures, prefer_const_literals_to_create_immutables, unused_field

import 'dart:async';
import 'dart:convert';
import 'dart:ffi' as ffi;

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated_io.dart';

import 'api/full.dart';
import 'frb_generated.dart';
import 'models/card.dart';
import 'models/config.dart';
import 'models/disclosure.dart';
import 'models/instruction.dart';
import 'models/pin.dart';
import 'models/uri.dart';
import 'models/version_state.dart';
import 'models/wallet_event.dart';

abstract class WalletCoreApiImplPlatform extends BaseApiImpl<WalletCoreWire> {
  WalletCoreApiImplPlatform({
    required super.handler,
    required super.wire,
    required super.generalizedFrbRustBinding,
    required super.portManager,
  });

  @protected
  AnyhowException dco_decode_AnyhowException(raw);

  @protected
  int dco_decode_CastedPrimitive_u_64(raw);

  @protected
  RustStreamSink<bool> dco_decode_StreamSink_bool_Sse(raw);

  @protected
  RustStreamSink<FlutterConfiguration> dco_decode_StreamSink_flutter_configuration_Sse(raw);

  @protected
  RustStreamSink<FlutterVersionState> dco_decode_StreamSink_flutter_version_state_Sse(raw);

  @protected
  RustStreamSink<List<Card>> dco_decode_StreamSink_list_card_Sse(raw);

  @protected
  RustStreamSink<List<WalletEvent>> dco_decode_StreamSink_list_wallet_event_Sse(raw);

  @protected
  String dco_decode_String(raw);

  @protected
  AcceptDisclosureResult dco_decode_accept_disclosure_result(raw);

  @protected
  bool dco_decode_bool(raw);

  @protected
  Card dco_decode_box_autoadd_card(raw);

  @protected
  Image dco_decode_box_autoadd_image(raw);

  @protected
  Organization dco_decode_box_autoadd_organization(raw);

  @protected
  RequestPolicy dco_decode_box_autoadd_request_policy(raw);

  @protected
  WalletInstructionError dco_decode_box_autoadd_wallet_instruction_error(raw);

  @protected
  Card dco_decode_card(raw);

  @protected
  CardAttribute dco_decode_card_attribute(raw);

  @protected
  CardPersistence dco_decode_card_persistence(raw);

  @protected
  CardValue dco_decode_card_value(raw);

  @protected
  DisclosureCard dco_decode_disclosure_card(raw);

  @protected
  DisclosureSessionType dco_decode_disclosure_session_type(raw);

  @protected
  DisclosureStatus dco_decode_disclosure_status(raw);

  @protected
  DisclosureType dco_decode_disclosure_type(raw);

  @protected
  FlutterConfiguration dco_decode_flutter_configuration(raw);

  @protected
  FlutterVersionState dco_decode_flutter_version_state(raw);

  @protected
  GenderCardValue dco_decode_gender_card_value(raw);

  @protected
  int dco_decode_i_32(raw);

  @protected
  IdentifyUriResult dco_decode_identify_uri_result(raw);

  @protected
  Image dco_decode_image(raw);

  @protected
  List<Card> dco_decode_list_card(raw);

  @protected
  List<CardAttribute> dco_decode_list_card_attribute(raw);

  @protected
  List<DisclosureCard> dco_decode_list_disclosure_card(raw);

  @protected
  List<LocalizedString> dco_decode_list_localized_string(raw);

  @protected
  List<MissingAttribute> dco_decode_list_missing_attribute(raw);

  @protected
  Uint8List dco_decode_list_prim_u_8_strict(raw);

  @protected
  List<WalletEvent> dco_decode_list_wallet_event(raw);

  @protected
  LocalizedString dco_decode_localized_string(raw);

  @protected
  MissingAttribute dco_decode_missing_attribute(raw);

  @protected
  int? dco_decode_opt_CastedPrimitive_u_64(raw);

  @protected
  String? dco_decode_opt_String(raw);

  @protected
  Image? dco_decode_opt_box_autoadd_image(raw);

  @protected
  List<DisclosureCard>? dco_decode_opt_list_disclosure_card(raw);

  @protected
  List<LocalizedString>? dco_decode_opt_list_localized_string(raw);

  @protected
  Organization dco_decode_organization(raw);

  @protected
  PinValidationResult dco_decode_pin_validation_result(raw);

  @protected
  RequestPolicy dco_decode_request_policy(raw);

  @protected
  StartDisclosureResult dco_decode_start_disclosure_result(raw);

  @protected
  int dco_decode_u_16(raw);

  @protected
  BigInt dco_decode_u_64(raw);

  @protected
  int dco_decode_u_8(raw);

  @protected
  void dco_decode_unit(raw);

  @protected
  WalletEvent dco_decode_wallet_event(raw);

  @protected
  WalletInstructionError dco_decode_wallet_instruction_error(raw);

  @protected
  WalletInstructionResult dco_decode_wallet_instruction_result(raw);

  @protected
  AnyhowException sse_decode_AnyhowException(SseDeserializer deserializer);

  @protected
  int sse_decode_CastedPrimitive_u_64(SseDeserializer deserializer);

  @protected
  RustStreamSink<bool> sse_decode_StreamSink_bool_Sse(SseDeserializer deserializer);

  @protected
  RustStreamSink<FlutterConfiguration> sse_decode_StreamSink_flutter_configuration_Sse(SseDeserializer deserializer);

  @protected
  RustStreamSink<FlutterVersionState> sse_decode_StreamSink_flutter_version_state_Sse(SseDeserializer deserializer);

  @protected
  RustStreamSink<List<Card>> sse_decode_StreamSink_list_card_Sse(SseDeserializer deserializer);

  @protected
  RustStreamSink<List<WalletEvent>> sse_decode_StreamSink_list_wallet_event_Sse(SseDeserializer deserializer);

  @protected
  String sse_decode_String(SseDeserializer deserializer);

  @protected
  AcceptDisclosureResult sse_decode_accept_disclosure_result(SseDeserializer deserializer);

  @protected
  bool sse_decode_bool(SseDeserializer deserializer);

  @protected
  Card sse_decode_box_autoadd_card(SseDeserializer deserializer);

  @protected
  Image sse_decode_box_autoadd_image(SseDeserializer deserializer);

  @protected
  Organization sse_decode_box_autoadd_organization(SseDeserializer deserializer);

  @protected
  RequestPolicy sse_decode_box_autoadd_request_policy(SseDeserializer deserializer);

  @protected
  WalletInstructionError sse_decode_box_autoadd_wallet_instruction_error(SseDeserializer deserializer);

  @protected
  Card sse_decode_card(SseDeserializer deserializer);

  @protected
  CardAttribute sse_decode_card_attribute(SseDeserializer deserializer);

  @protected
  CardPersistence sse_decode_card_persistence(SseDeserializer deserializer);

  @protected
  CardValue sse_decode_card_value(SseDeserializer deserializer);

  @protected
  DisclosureCard sse_decode_disclosure_card(SseDeserializer deserializer);

  @protected
  DisclosureSessionType sse_decode_disclosure_session_type(SseDeserializer deserializer);

  @protected
  DisclosureStatus sse_decode_disclosure_status(SseDeserializer deserializer);

  @protected
  DisclosureType sse_decode_disclosure_type(SseDeserializer deserializer);

  @protected
  FlutterConfiguration sse_decode_flutter_configuration(SseDeserializer deserializer);

  @protected
  FlutterVersionState sse_decode_flutter_version_state(SseDeserializer deserializer);

  @protected
  GenderCardValue sse_decode_gender_card_value(SseDeserializer deserializer);

  @protected
  int sse_decode_i_32(SseDeserializer deserializer);

  @protected
  IdentifyUriResult sse_decode_identify_uri_result(SseDeserializer deserializer);

  @protected
  Image sse_decode_image(SseDeserializer deserializer);

  @protected
  List<Card> sse_decode_list_card(SseDeserializer deserializer);

  @protected
  List<CardAttribute> sse_decode_list_card_attribute(SseDeserializer deserializer);

  @protected
  List<DisclosureCard> sse_decode_list_disclosure_card(SseDeserializer deserializer);

  @protected
  List<LocalizedString> sse_decode_list_localized_string(SseDeserializer deserializer);

  @protected
  List<MissingAttribute> sse_decode_list_missing_attribute(SseDeserializer deserializer);

  @protected
  Uint8List sse_decode_list_prim_u_8_strict(SseDeserializer deserializer);

  @protected
  List<WalletEvent> sse_decode_list_wallet_event(SseDeserializer deserializer);

  @protected
  LocalizedString sse_decode_localized_string(SseDeserializer deserializer);

  @protected
  MissingAttribute sse_decode_missing_attribute(SseDeserializer deserializer);

  @protected
  int? sse_decode_opt_CastedPrimitive_u_64(SseDeserializer deserializer);

  @protected
  String? sse_decode_opt_String(SseDeserializer deserializer);

  @protected
  Image? sse_decode_opt_box_autoadd_image(SseDeserializer deserializer);

  @protected
  List<DisclosureCard>? sse_decode_opt_list_disclosure_card(SseDeserializer deserializer);

  @protected
  List<LocalizedString>? sse_decode_opt_list_localized_string(SseDeserializer deserializer);

  @protected
  Organization sse_decode_organization(SseDeserializer deserializer);

  @protected
  PinValidationResult sse_decode_pin_validation_result(SseDeserializer deserializer);

  @protected
  RequestPolicy sse_decode_request_policy(SseDeserializer deserializer);

  @protected
  StartDisclosureResult sse_decode_start_disclosure_result(SseDeserializer deserializer);

  @protected
  int sse_decode_u_16(SseDeserializer deserializer);

  @protected
  BigInt sse_decode_u_64(SseDeserializer deserializer);

  @protected
  int sse_decode_u_8(SseDeserializer deserializer);

  @protected
  void sse_decode_unit(SseDeserializer deserializer);

  @protected
  WalletEvent sse_decode_wallet_event(SseDeserializer deserializer);

  @protected
  WalletInstructionError sse_decode_wallet_instruction_error(SseDeserializer deserializer);

  @protected
  WalletInstructionResult sse_decode_wallet_instruction_result(SseDeserializer deserializer);

  @protected
  void sse_encode_AnyhowException(AnyhowException self, SseSerializer serializer);

  @protected
  void sse_encode_CastedPrimitive_u_64(int self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_bool_Sse(RustStreamSink<bool> self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_flutter_configuration_Sse(
      RustStreamSink<FlutterConfiguration> self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_flutter_version_state_Sse(
      RustStreamSink<FlutterVersionState> self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_list_card_Sse(RustStreamSink<List<Card>> self, SseSerializer serializer);

  @protected
  void sse_encode_StreamSink_list_wallet_event_Sse(RustStreamSink<List<WalletEvent>> self, SseSerializer serializer);

  @protected
  void sse_encode_String(String self, SseSerializer serializer);

  @protected
  void sse_encode_accept_disclosure_result(AcceptDisclosureResult self, SseSerializer serializer);

  @protected
  void sse_encode_bool(bool self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_card(Card self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_image(Image self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_organization(Organization self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_request_policy(RequestPolicy self, SseSerializer serializer);

  @protected
  void sse_encode_box_autoadd_wallet_instruction_error(WalletInstructionError self, SseSerializer serializer);

  @protected
  void sse_encode_card(Card self, SseSerializer serializer);

  @protected
  void sse_encode_card_attribute(CardAttribute self, SseSerializer serializer);

  @protected
  void sse_encode_card_persistence(CardPersistence self, SseSerializer serializer);

  @protected
  void sse_encode_card_value(CardValue self, SseSerializer serializer);

  @protected
  void sse_encode_disclosure_card(DisclosureCard self, SseSerializer serializer);

  @protected
  void sse_encode_disclosure_session_type(DisclosureSessionType self, SseSerializer serializer);

  @protected
  void sse_encode_disclosure_status(DisclosureStatus self, SseSerializer serializer);

  @protected
  void sse_encode_disclosure_type(DisclosureType self, SseSerializer serializer);

  @protected
  void sse_encode_flutter_configuration(FlutterConfiguration self, SseSerializer serializer);

  @protected
  void sse_encode_flutter_version_state(FlutterVersionState self, SseSerializer serializer);

  @protected
  void sse_encode_gender_card_value(GenderCardValue self, SseSerializer serializer);

  @protected
  void sse_encode_i_32(int self, SseSerializer serializer);

  @protected
  void sse_encode_identify_uri_result(IdentifyUriResult self, SseSerializer serializer);

  @protected
  void sse_encode_image(Image self, SseSerializer serializer);

  @protected
  void sse_encode_list_card(List<Card> self, SseSerializer serializer);

  @protected
  void sse_encode_list_card_attribute(List<CardAttribute> self, SseSerializer serializer);

  @protected
  void sse_encode_list_disclosure_card(List<DisclosureCard> self, SseSerializer serializer);

  @protected
  void sse_encode_list_localized_string(List<LocalizedString> self, SseSerializer serializer);

  @protected
  void sse_encode_list_missing_attribute(List<MissingAttribute> self, SseSerializer serializer);

  @protected
  void sse_encode_list_prim_u_8_strict(Uint8List self, SseSerializer serializer);

  @protected
  void sse_encode_list_wallet_event(List<WalletEvent> self, SseSerializer serializer);

  @protected
  void sse_encode_localized_string(LocalizedString self, SseSerializer serializer);

  @protected
  void sse_encode_missing_attribute(MissingAttribute self, SseSerializer serializer);

  @protected
  void sse_encode_opt_CastedPrimitive_u_64(int? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_String(String? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_box_autoadd_image(Image? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_list_disclosure_card(List<DisclosureCard>? self, SseSerializer serializer);

  @protected
  void sse_encode_opt_list_localized_string(List<LocalizedString>? self, SseSerializer serializer);

  @protected
  void sse_encode_organization(Organization self, SseSerializer serializer);

  @protected
  void sse_encode_pin_validation_result(PinValidationResult self, SseSerializer serializer);

  @protected
  void sse_encode_request_policy(RequestPolicy self, SseSerializer serializer);

  @protected
  void sse_encode_start_disclosure_result(StartDisclosureResult self, SseSerializer serializer);

  @protected
  void sse_encode_u_16(int self, SseSerializer serializer);

  @protected
  void sse_encode_u_64(BigInt self, SseSerializer serializer);

  @protected
  void sse_encode_u_8(int self, SseSerializer serializer);

  @protected
  void sse_encode_unit(void self, SseSerializer serializer);

  @protected
  void sse_encode_wallet_event(WalletEvent self, SseSerializer serializer);

  @protected
  void sse_encode_wallet_instruction_error(WalletInstructionError self, SseSerializer serializer);

  @protected
  void sse_encode_wallet_instruction_result(WalletInstructionResult self, SseSerializer serializer);
}

// Section: wire_class

class WalletCoreWire implements BaseWire {
  /// The symbols are looked up in [dynamicLibrary].
  WalletCoreWire(ffi.DynamicLibrary dynamicLibrary) : _lookup = dynamicLibrary.lookup;

  factory WalletCoreWire.fromExternalLibrary(ExternalLibrary lib) => WalletCoreWire(lib.ffiDynamicLibrary);

  /// Holds the symbol lookup function.
  final ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName) _lookup;
}
