﻿﻿﻿﻿﻿/** A command line utility to send/receive requests from/to AudaBridge.
 * 
 * 
 * */


using System;
using System.Diagnostics;
using AimsPresentation;﻿
using System.Xml;
using System.IO;

namespace AudaBridge
{
    class AudaBridgeComms
    {

        #region Vars and Constants

        static AimsInterfaces.IxmlDoc xJ;
        static string sEndPoint; //Auda soap interface endpoint (url)
        static string sFolderIn, sFolderOut, sFolderSent, sFolderFailed; //folders under \bin\ for auda files
        //static string sTerminalID; //auda terminal id for AIMS from the config file
        static bool bLoggingEnabled; // log all soap traffic if set to true
        public static bool bLogEnvelope; //log the envelope, which can be redundant past establishing the connection
        static BackOffice.CredentialsInfo oCreds = Init(); //initialises vars and settings
        //list of files in the sent folder for reference validation
        static System.Collections.Specialized.StringCollection arSentFiles = new System.Collections.Specialized.StringCollection(); //list of file names in auda-sent folder

        const int EmptyQueueErrorCode = 4101; // the error code returned by Auda if there are no messages in the queue

        /// <summary>
        /// Auda msg types to place in the header of every SOAP request
        /// </summary>
        enum msgTypes
        {
            MSGTYPE_PING, MSGTYPE_ASSESSMENT, MSGTYPE_CONF, MSGTYPE_MAIL, MSGTYPE_MAIL_EXT, MSGTYPE_MAIL_ADMIN,
            MSGTYPE_MAIL_INTADV, MSGTYPE_MAIL_AUTH, MSGTYPE_IMAGE
        }

        /// <summary>
        /// Contains all message parts in Auda responses, assessments, confirmations, etc. Not all fields are used by all messages.
        /// </summary>
        struct stDetailedMsg
        {
            public string MessageTypeIdentifier, AssessmentNumber, Originator, MsgSequence, MsgCreatedTime, AimsReference, InstrResultCode, InstrResultMessage;
            public int iAckMsgId, iQueueDepth, iErrorCode;
            public bool ResponseRecieved;
            public string Body; //the actual payload
            public string Envelope; //MsgData envelope without the body part
            public string FileName; //it only has a value if there is a body to save
        }

        #endregion


        static void Main(string[] args)
        {
            //LogIt(String.Format(TextMsgs.msgAppStarted, bLoggingEnabled, bLogEnvelope), System.Diagnostics.EventLogEntryType.Information);

            sendOutgoing();

            getConfirmations();

            getAssessments();

            getImages();

            //LogIt(TextMsgs.msgAppFinished, System.Diagnostics.EventLogEntryType.Information);

            //Comment this out to run unattended, but should be no harm keeping it on
            System.Threading.Thread.Sleep(10000);
        }


        #region Queue operations

        /// <summary>
        /// Send out all messages from the local outgoing folder to Audatex queue
        /// </summary>
        /// <param name="sFolderOut"></param>
        static void sendOutgoing()
        {

            //get the list of files
            string[] arFiles = System.IO.Directory.GetFiles(sFolderOut);
            int iFilesToProcess = arFiles.Length;
            //LogIt(String.Format(TextMsgs.msgOutgoing, iFilesToProcess), System.Diagnostics.EventLogEntryType.Information);
            if (iFilesToProcess == 0) return;

            //loop thru the files
            foreach (string sFullFileName in arFiles)
            {
                //read the file
                string sFileNameNoPath = System.IO.Path.GetFileName(sFullFileName);

                //prepare the request
                string sEnvelope = PrepareEnvelope(sFullFileName, msgTypes.MSGTYPE_MAIL);
                //sEnvelope = TextMsgs.soapTest;
                BackOffice.PutDataRequestBody soapBody = new BackOffice.PutDataRequestBody(oCreds, msgTypes.MSGTYPE_MAIL.ToString(), sEnvelope);
                BackOffice.PutDataRequest soapReq = new BackOffice.PutDataRequest(soapBody);
                BackOffice.BackOfficeWebServiceSoapClient soapClient = new BackOffice.BackOfficeWebServiceSoapClient("BackOfficeWebServiceSoap", sEndPoint);
                BackOffice.PutDataResponse soapResp = null;

                if (bLoggingEnabled) soapClient.Endpoint.EndpointBehaviors.Add(new AudaBridge.DebugMessageBehavior());

                //put data
                try
                {
                    soapResp = soapClient.PutData(soapReq);
                }
                catch (Exception ex)
                {
                    LogIt(ex.Message + ex.StackTrace, System.Diagnostics.EventLogEntryType.Error);
                    break;
                }

                //extract data from the response
                BackOffice.PutDataResult soapResult = soapResp.Body.PutDataResult;
                String sErrorMsg = soapResult.ErrorMessage + " " + soapResult.AdditionalData;
                int iErrorCode = soapResult.ErrorCode;

                // Note, the response is very limited. It only says that the message was received and looks like a message.
                // The processing will be done asynchronously and the results placed in _CONF queue.

                //report
                if (iErrorCode == 0)
                {
                    //success
                    //LogIt(string.Format(TextMsgs.msgSuccess, sFullFileName), System.Diagnostics.EventLogEntryType.Information);
                    try
                    {
                        System.IO.File.Move(sFullFileName, sFolderSent + sFileNameNoPath); //move into sent folder to wait for _CONF message
                    }
                    catch (Exception ex)
                    {
                        LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error);
                    }
                }
                else
                {
                    //failure
                    LogIt(string.Format(TextMsgs.msgFailure, sFullFileName, sErrorMsg), System.Diagnostics.EventLogEntryType.Warning);
                    try
                    {
                        System.IO.File.Move(sFullFileName, sFolderFailed + sFileNameNoPath); //move into failed folder for admins to sort out
                    }
                    catch (Exception ex)
                    {
                        LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error);
                    }
                }
            }

            //get the list of files in the outgoing folder for future validation
            arSentFiles.AddRange(System.IO.Directory.GetFiles(sFolderSent));
        }



        /// <summary>
        /// A confirmation message is available in a separate queue after the submission was processed by Auda.
        /// </summary>
        static void getConfirmations()
        {
            //LogIt(TextMsgs.msgConfirmations, System.Diagnostics.EventLogEntryType.Information);

            //get list of files awaiting confirmation
            foreach (string sFile in Directory.GetFiles(sFolderSent))
            {
                arSentFiles.Add(Path.GetFileName(sFile));
            }

            int iAckMsgId = 0; //id of the previous confirmation msg to be removed from the queue
            int iQueueDepth = 1; //initial value to start the loop
            int iQueueDepthPrev = -1; //the previous value of the queue depth for progress monitoring 

            //loop thru the files
            while (iQueueDepth > 0)
            {
                stDetailedMsg stResult = getMsgFromQueue(iAckMsgId, msgTypes.MSGTYPE_CONF);

                //Check the state of the response and if the loop should be exited
                iAckMsgId = stResult.iAckMsgId;
                iQueueDepth = (iQueueDepthPrev == stResult.iQueueDepth) ? iQueueDepth - 1 : stResult.iQueueDepth; //make sure there is no infinite loop and advance by 1 even if the remove queue hasn't progressed
                iQueueDepthPrev = stResult.iQueueDepth;
                if (stResult.iQueueDepth == 0 || !stResult.ResponseRecieved) break; //there was either an error or we reached the end of the queue

                //the file name was used as a reference
                string sFileName = stResult.AimsReference;

                //report
                if (stResult.InstrResultCode == "0")
                {
                    //success
                    //LogIt(string.Format(TextMsgs.msgConfirmed, stResult.AssessmentNumber), System.Diagnostics.EventLogEntryType.Information);
                    if (sFileName != "")
                    {
                        //save the body of the response to get the assessment number for the claim. It will be imported into the DB later for response matching.
                        try { System.IO.File.WriteAllText(sFolderIn + sFileName, stResult.Envelope); }
                        catch (Exception ex) { LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error); }
                        //don't really need to keep the file if it's now in AEG
                        try { System.IO.File.Delete(sFolderSent + sFileName); }
                        catch (Exception ex) { LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error); }
                    }
                }
                else
                {
                    //failure
                    LogIt(string.Format(TextMsgs.msgFailure, sFileName, stResult.InstrResultMessage), System.Diagnostics.EventLogEntryType.Warning);
                    try
                    {
                        if (sFileName != "")
                        {
                            System.IO.File.Move(sFolderSent + sFileName, sFolderFailed + sFileName); //for the admins to sort out manually
                            System.IO.File.SetLastWriteTime(sFolderFailed + sFileName, DateTime.Now); //the date/time update helps link them raw SOAP dumps
                        }
                    }
                    catch (Exception ex)
                    {
                        LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error);
                    }
                }
            }
        }


        /// <summary>
        /// Assessments are read from an Auda queue and dumped into an incoming folder locally for further processing by AIMS later.
        /// </summary>
        static void getAssessments()
        {
            //LogIt(TextMsgs.msgAssessments, System.Diagnostics.EventLogEntryType.Information);

            int iAckMsgId = 0; //id of the previous assessment msg to be removed from the queue
            int iQueueDepth = 1; //initial value to start the loop
            int iQueueDepthPrev = -1; //the previous value of the queue depth for progress monitoring 
                                      //loop thru the files
            while (iQueueDepth > 0)
            {
                //Get the next msg from the queue
                stDetailedMsg stResult = getMsgFromQueue(iAckMsgId, msgTypes.MSGTYPE_ASSESSMENT);

                //Check the state of the response and if the loop should be exited
                iAckMsgId = stResult.iAckMsgId;
                iQueueDepth = (iQueueDepthPrev == stResult.iQueueDepth) ? iQueueDepth - 1 : stResult.iQueueDepth; //make sure there is no infinite loop and advance by 1 even if the remove queue hasn't progressed
                iQueueDepthPrev = stResult.iQueueDepth;
                if (stResult.iQueueDepth == 0 || !stResult.ResponseRecieved) break; //there was either an error or we reached the end of the queue

                //the file name is random, not linked to the claim (may need to change that)
                string sFileName = stResult.FileName;

                //save as a file
                //LogIt(string.Format(TextMsgs.msgAssessmentReceived, stResult.AssessmentNumber), System.Diagnostics.EventLogEntryType.Information);
                if (stResult.Body == "") LogIt(string.Format(TextMsgs.msgEmptyMessage, stResult.AssessmentNumber), System.Diagnostics.EventLogEntryType.Error);
                try
                {
                    if (sFileName != "") System.IO.File.WriteAllText(sFolderIn + sFileName, stResult.Body);
                }
                catch (Exception ex)
                {
                    LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error);
                }
            }
        }


        /// <summary>
        /// Get all images from the image queue and save them in auda-in folder as .jpeg + .auda meta file
        /// </summary>
        static void getImages()
        {
            //LogIt(TextMsgs.msgImages, System.Diagnostics.EventLogEntryType.Information);

            int iAckMsgId = 0; //id of the previous confirmation msg to be removed from the queue
            int iQueueDepth = 1; //initial value to start the loop
            int iQueueDepthPrev = -1; //the previous value of the queue depth for progress monitoring 

            //loop thru the files
            while (iQueueDepth > 0)
            {

                stDetailedMsg stResult = getMsgFromQueue(iAckMsgId, msgTypes.MSGTYPE_IMAGE);

                //Check the state of the response and if the loop should be exited
                iAckMsgId = stResult.iAckMsgId;
                iQueueDepth = (iQueueDepthPrev == stResult.iQueueDepth) ? iQueueDepth - 1 : stResult.iQueueDepth; //make sure there is no infinite loop and advance by 1 even if the remove queue hasn't progressed
                iQueueDepthPrev = stResult.iQueueDepth;
                if (stResult.iQueueDepth == 0 || !stResult.ResponseRecieved) break; //there was either an error or we reached the end of the queue

                //the file name is random, not linked to the claim (may need to change that)
                string sFileName = stResult.FileName;

                //check if we have the data
                //LogIt(string.Format(TextMsgs.msgAssessmentReceived, stResult.AssessmentNumber), System.Diagnostics.EventLogEntryType.Information);
                if (stResult.Body == "" || stResult.Body == null || stResult.iErrorCode != 0)
                {
                    LogIt(string.Format(TextMsgs.msgEmptyMessage, stResult.AimsReference), System.Diagnostics.EventLogEntryType.Error);
                    continue; //got something in the response, but it's broken. Move to the next record.
                }

                //Save the base64 data as a jpeg file
                System.Xml.XmlDocument xImg = new System.Xml.XmlDocument();
                xImg.LoadXml(stResult.Body);
                System.Xml.XmlNode xI = xImg.DocumentElement.SelectSingleNode("AudaImageContent/Image");
                byte[] bytes = Convert.FromBase64String(xI.InnerText);
                FileStream imageFile = new FileStream(sFolderIn + System.IO.Path.GetFileNameWithoutExtension(sFileName) + ".jpg", FileMode.Create);
                imageFile.Write(bytes, 0, bytes.Length);
                imageFile.Flush();
                imageFile.Close();

                //save the metadata with the same file name, no .jpg ext
                //These 2 nodes act as unique ID for an assessment. It is possible that an assessment # is
                //duplicated, but that should not happen per user, which is the ORIGINATOR.
                xI.InnerText = "";
                xImg.DocumentElement.SetAttribute("AssessmentNumber", stResult.AssessmentNumber);
                xImg.DocumentElement.SetAttribute("Originator", stResult.Originator);
                try
                {
                    if (sFileName != "") File.WriteAllText(sFolderIn + Path.GetFileNameWithoutExtension(sFileName) + AimsInterfaces.Paths4.FileExtensionAudaImg, xImg.DocumentElement.OuterXml);
                }
                catch (Exception ex)
                {
                    LogIt(ex.Message, System.Diagnostics.EventLogEntryType.Error);
                }
            }

            //}
        }



        /// <summary>
        /// Downloads msgs from the specified queue and returns the results in a structure. 
        /// </summary>
        /// <param name="iAckMsgId"></param>
        /// <param name="msgType"></param>
        /// <returns></returns>
        static stDetailedMsg getMsgFromQueue(int iAckMsgId, msgTypes msgType)


        {

            stDetailedMsg stResult = new stDetailedMsg();
            stResult.ResponseRecieved = false; //indicates no response in case of early return from this function

            //prep the request
            BackOffice.GetDataRequestBody soapBody = new BackOffice.GetDataRequestBody(oCreds, msgType.ToString(), iAckMsgId);
            BackOffice.GetDataRequest soapReq = new BackOffice.GetDataRequest(soapBody);
            BackOffice.BackOfficeWebServiceSoapClient soapClient = new BackOffice.BackOfficeWebServiceSoapClient("BackOfficeWebServiceSoap", sEndPoint);
            BackOffice.GetDataResponse soapResp = null;

            //comment-uncomment to log raw SOAP messages
            if (bLoggingEnabled) soapClient.Endpoint.EndpointBehaviors.Add(new AudaBridge.DebugMessageBehavior());

            //get data
            try
            {
                soapResp = soapClient.GetData(soapReq);
            }
            catch (Exception ex)
            {
                LogIt(ex.Message + ex.StackTrace, System.Diagnostics.EventLogEntryType.Error);
                return stResult;
            }

            //extract data from the response
            BackOffice.GetDataResult soapResult = soapResp.Body.GetDataResult;
            String sErrorMsg = soapResult.ErrorMessage;
            stResult.iErrorCode = soapResult.ErrorCode;
            stResult.iAckMsgId = soapResult.MessageId;
            stResult.iQueueDepth = soapResult.QueueDepth;

            stResult.ResponseRecieved = true; //indicates to the caller that the response was recieved

            //check if it was an empty run
            if (stResult.iErrorCode == EmptyQueueErrorCode && stResult.iQueueDepth == 0) return stResult;

            //there can be more data in the message envelope
            try
            {
                stResult = readConfMsgResponse(stResult, soapResult.MessageEnvelope);
            }
            catch (Exception)
            {
                LogIt(string.Format(TextMsgs.msgInvalidResponse, soapResult.MessageEnvelope, iAckMsgId), System.Diagnostics.EventLogEntryType.Warning);
                // no point processing from now on. The msg will be removed from the queueu
            }

            return stResult;
        }

        #endregion


        #region Helpers


        /// <summary>
        /// Wraps sProcessor.LogIt to add concole output
        /// </summary>
        /// <param name="EventText"></param>
        /// <param name="EntryType"></param>
        public static void LogIt(string EventText, System.Diagnostics.EventLogEntryType EntryType = System.Diagnostics.EventLogEntryType.Error)
        {
            Console.WriteLine(EventText);
            sProcessor.LogIt(EventText, EntryType);
        }


        /// <summary>
        /// Put the SOAP body in an envelope and set some other params.
        /// </summary>
        /// <param name="BodyContents"></param>
        /// <returns></returns>
        private static string PrepareEnvelope(string sFileName, msgTypes msgType)
        {
            string sFileContents = System.IO.File.ReadAllText(sFileName);
            string sReference = System.IO.Path.GetFileNameWithoutExtension(sFileName);

            System.Xml.XmlDocument xEnv = new System.Xml.XmlDocument();
            xEnv.LoadXml(TextMsgs.soapEnvelope);

            ((System.Xml.XmlElement)xEnv.DocumentElement.SelectSingleNode("Header/MsgTypeIdentifier")).InnerText = msgType.ToString();
            ((System.Xml.XmlElement)xEnv.DocumentElement.SelectSingleNode("Header/MsgCreatedTime")).InnerText = DateTime.Now.ToString("s");
            ((System.Xml.XmlElement)xEnv.DocumentElement.SelectSingleNode("Header/Reference")).InnerXml = sReference;
            ((System.Xml.XmlElement)xEnv.DocumentElement.SelectSingleNode("Body")).InnerXml = sFileContents;

            return xEnv.DocumentElement.OuterXml;

        }


        /// <summary>
        /// Initialise vars and get creds for connecting to AudaBridge
        /// </summary>
        /// <returns></returns>
        static BackOffice.CredentialsInfo Init()
        {
            //Get paths
            string sCurrentPath = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetExecutingAssembly().Location) + "\\";

            //Load initial config settings
            xJ = new AimsInterfaces.IxmlDoc();
            sEndPoint = xJ.Setting("auda-endpoint");
            sFolderIn =  xJ.Setting("webcomms-import") + "\\"; //sCurrentPath + AimsInterfaces.PathsAuda.Folders.QueueIn + "\\"; can be replaced when GMX is decomissioned
            sFolderOut = sCurrentPath + AimsInterfaces.PathsAuda.Folders.QueueOut + "\\";
            sFolderSent = sCurrentPath + AimsInterfaces.PathsAuda.Folders.QueueSent + "\\";
            sFolderFailed = sCurrentPath + AimsInterfaces.PathsAuda.Folders.QueueFailed + "\\";
            //sTerminalID = xJ.Setting("auda-terminalid");

            //check logging level
            string sLogLevel = xJ.Setting("auda-logging");
            switch (sLogLevel) {
                case "1":
                    bLoggingEnabled = true;
                    bLogEnvelope = false;
                    break;
                case "2":
                    bLoggingEnabled = true;
                    bLogEnvelope = true;
                    break;
                default:
                    bLoggingEnabled = false;
                    bLogEnvelope = false;
                    break;
            }

            //load creds from the config
            BackOffice.CredentialsInfo oCreds = new BackOffice.CredentialsInfo();
            oCreds.CompanyCode = xJ.Setting("auda-company");
            oCreds.UserId = xJ.Setting("auda-userid");
            oCreds.Password = xJ.Setting("auda-pwd");

            return oCreds;
        }


        /// <summary>
        /// Deserialises a confirmation msg response from XML into a structure.
        /// </summary>
        /// <param name="MessageEnvelope"></param>
        /// <param name="metaName">Add an attribute  with this name to the root element</param>
        /// <param name="metaValue">Add an attribute with this value to the root element</param>
        /// <returns></returns>
        static stDetailedMsg readConfMsgResponse(stDetailedMsg stResult, string MessageEnvelope)
        {


            System.Xml.XmlDocument xEnv = new System.Xml.XmlDocument();
            xEnv.LoadXml(MessageEnvelope);
            System.Xml.XmlElement xD = xEnv.DocumentElement;

            System.Xml.XmlNode xN = xD.SelectSingleNode("Header/MsgSequence");
            if (xN != null) stResult.MsgSequence = xN.InnerText;

            xN = xD.SelectSingleNode("Header/Reference");
            if (xN != null) stResult.AimsReference = xN.InnerText + AimsInterfaces.Paths4.FileExtensionAuda;
            if (!arSentFiles.Contains(stResult.AimsReference)) stResult.AimsReference = ""; //validate the reference because it will be used in a file path

            xN = xD.SelectSingleNode("Header/InstrResultCode");
            if (xN != null) stResult.InstrResultCode = xN.InnerText;

            xN = xD.SelectSingleNode("Header/InstrResultMessage");
            if (xN != null) stResult.InstrResultMessage = xN.InnerText;

            xN = xD.SelectSingleNode("Header/MsgCreatedTime");
            if (xN != null) stResult.MsgCreatedTime = xN.InnerText;


            xN = xD.SelectSingleNode("Header/Originator");
            if (xN != null) stResult.Originator = xN.InnerText;

            xN = xD.SelectSingleNode("Header/MsgTypeIdentifier");
            if (xN != null) stResult.MessageTypeIdentifier = xN.InnerText;

            xN = xD.SelectSingleNode("Header/AssessmentNumber");
            if (xN != null) stResult.AssessmentNumber = xN.InnerText;


            xN = xD.SelectSingleNode("Body");
            if (xN != null)
            {
                //save in the structure
                stResult.Body = xN.InnerXml;
                stResult.FileName = Guid.NewGuid().ToString("N") + AimsInterfaces.Paths4.FileExtensionAuda;
                xN.InnerXml = ""; //remove the body now to save the envelope
            }

            stResult.Envelope = xD.OuterXml; //save the envelope itself


            //load the body as XML to get more data
            if (stResult.Body != "")
            {
                System.Xml.XmlDocument xBody = new System.Xml.XmlDocument();
                try
                {
                    xBody.LoadXml(stResult.Body);
                }
                catch
                {
                    LogIt(TextMsgs.msgBodyInvalid, System.Diagnostics.EventLogEntryType.Warning);
                    return stResult;
                }

            }

            return stResult;
        }


        #endregion

    }
}
